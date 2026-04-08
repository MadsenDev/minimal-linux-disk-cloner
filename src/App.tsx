import { useEffect, useMemo, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { createPortal } from "react-dom";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  HiArrowPath,
  HiBolt,
  HiCheckCircle,
  HiCheck,
  HiChevronDown,
  HiCircleStack,
  HiCog6Tooth,
  HiDocumentArrowDown,
  HiExclamationTriangle,
  HiInformationCircle,
  HiServerStack,
  HiXCircle,
} from "react-icons/hi2";
import clsx from "clsx";
import logoUrl from "../assets/mldc.svg";

type WarningSeverity = "Info" | "Warning" | "Danger";

type PartitionInfo = {
  path: string;
  mountpoint: string | null;
  fstype: string | null;
};

type DiskDevice = {
  path: string;
  name: string;
  size_bytes: number;
  size_human: string;
  model: string | null;
  serial: string | null;
  removable: boolean;
  transport: string | null;
  mounted_partitions: PartitionInfo[];
};

type WarningItem = {
  severity: WarningSeverity;
  code: string;
  message: string;
};

type RuntimeStatus = {
  mode: "real" | "fake";
  is_root: boolean;
  has_lsblk: boolean;
  has_dd: boolean;
  errors: string[];
};

type RunPhase = "clone" | "verify";

type CloneProgress = {
  phase: RunPhase;
  bytes_copied: number;
  total_bytes: number;
  percent: number;
  speed_bytes_per_sec: number | null;
  elapsed_secs: number;
  eta_secs: number | null;
  raw_output: string[];
};

type CloneResult = {
  phase: RunPhase;
  success: boolean;
  bytes_copied: number;
  elapsed_secs: number;
  average_speed: number | null;
  final_message: string;
  verify_requested: boolean;
  verify_completed: boolean;
  raw_output: string[];
};

type StartCloneResponse = {
  run_id: string;
  command_preview: string;
  total_bytes: number;
  verify_after_clone: boolean;
};

type Phase = "setup" | "progress" | "result";

const stageBase =
  "flex w-full flex-col items-center text-center";
const VERIFY_DEFAULT_STORAGE_KEY = "mldc.verify-default";
const BACKGROUND_MOTION_STORAGE_KEY = "mldc.background-motion";

export default function App() {
  const [phase, setPhase] = useState<Phase>("setup");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [devices, setDevices] = useState<DiskDevice[]>([]);
  const [sourcePath, setSourcePath] = useState("");
  const [targetPath, setTargetPath] = useState("");
  const [warnings, setWarnings] = useState<WarningItem[]>([]);
  const [progress, setProgress] = useState<CloneProgress | null>(null);
  const [result, setResult] = useState<CloneResult | null>(null);
  const [commandPreview, setCommandPreview] = useState("");
  const [verifyAfterClone, setVerifyAfterClone] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [starting, setStarting] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [savingReport, setSavingReport] = useState(false);
  const [savedReportPath, setSavedReportPath] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false);
  const [showStopModal, setShowStopModal] = useState(false);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [defaultVerifyAfterClone, setDefaultVerifyAfterClone] = useState(false);
  const [backgroundMotionEnabled, setBackgroundMotionEnabled] = useState(true);
  const [appVersion, setAppVersion] = useState("0.1.0");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const source = useMemo(
    () => devices.find((device) => device.path === sourcePath) ?? null,
    [devices, sourcePath],
  );
  const target = useMemo(
    () => devices.find((device) => device.path === targetPath) ?? null,
    [devices, targetPath],
  );

  useEffect(() => {
    void refreshState();
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const storedVerifyDefault = window.localStorage.getItem(VERIFY_DEFAULT_STORAGE_KEY);
    const storedBackgroundMotion = window.localStorage.getItem(BACKGROUND_MOTION_STORAGE_KEY);

    const verifyDefault = storedVerifyDefault === "true";
    const motionEnabled = storedBackgroundMotion !== "false";

    setDefaultVerifyAfterClone(verifyDefault);
    setVerifyAfterClone(verifyDefault);
    setBackgroundMotionEnabled(motionEnabled);

    void getVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion("0.1.0"));
  }, []);

  useEffect(() => {
    if (!sourcePath || !targetPath) {
      setWarnings([]);
      return;
    }

    void invoke<WarningItem[]>("get_clone_warnings", {
      sourcePath,
      targetPath,
    })
      .then(setWarnings)
      .catch((error) => setErrorMessage(String(error)));
  }, [sourcePath, targetPath]);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;

    const setupListeners = async () => {
      unlistenProgress = await listen<CloneProgress>("clone-progress", (event) => {
        setProgress(event.payload);
        setPhase("progress");
      });

      unlistenFinished = await listen<CloneResult>("clone-finished", (event) => {
        setResult(event.payload);
        setProgress(null);
        setStarting(false);
        setStopping(false);
        setShowLogs(false);
        setPhase("result");
      });
    };

    void setupListeners();

    return () => {
      unlistenProgress?.();
      unlistenFinished?.();
    };
  }, []);

  async function refreshState() {
    setRefreshing(true);
    setErrorMessage(null);

    try {
      const [status, detectedDevices] = await Promise.all([
        invoke<RuntimeStatus>("get_runtime_status"),
        invoke<DiskDevice[]>("list_devices"),
      ]);

      setRuntimeStatus(status);
      setDevices(detectedDevices);

      if (!detectedDevices.some((device) => device.path === sourcePath)) {
        setSourcePath("");
      }
      if (!detectedDevices.some((device) => device.path === targetPath)) {
        setTargetPath("");
      }
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setRefreshing(false);
    }
  }

  async function beginClone() {
    if (!canClone) {
      return;
    }

    setStarting(true);
    setErrorMessage(null);
    setResult(null);
    setShowLogs(false);
    setSavedReportPath(null);

    try {
      const [latestStatus, latestDevices] = await Promise.all([
        invoke<RuntimeStatus>("get_runtime_status"),
        invoke<DiskDevice[]>("list_devices"),
      ]);

      setRuntimeStatus(latestStatus);
      setDevices(latestDevices);

      const latestSource = latestDevices.find((device) => device.path === sourcePath);
      const latestTarget = latestDevices.find((device) => device.path === targetPath);
      if (!latestSource || !latestTarget) {
        throw new Error(
          "The selected source or target changed before the clone started. Refresh and choose the devices again.",
        );
      }

      const latestWarnings = await invoke<WarningItem[]>("get_clone_warnings", {
        sourcePath,
        targetPath,
      });
      setWarnings(latestWarnings);
      if (latestWarnings.some((warning) => warning.code === "same-device")) {
        throw new Error("Source and target cannot be the same device.");
      }

      const response = await invoke<StartCloneResponse>("start_clone", {
        sourcePath,
        targetPath,
        verifyAfterClone,
      });
      setCommandPreview(response.command_preview);
      setPhase("progress");
      setProgress({
        phase: "clone",
        bytes_copied: 0,
        total_bytes: response.total_bytes,
        percent: 0,
        speed_bytes_per_sec: null,
        elapsed_secs: 0,
        eta_secs: null,
        raw_output: [],
      });
    } catch (error) {
      setStarting(false);
      setErrorMessage(String(error));
    }
  }

  async function stopClone() {
    setStopping(true);
    setShowStopModal(false);
    setErrorMessage(null);

    try {
      await invoke("stop_clone");
    } catch (error) {
      setErrorMessage(String(error));
      setStopping(false);
    }
  }

  async function saveRunReport() {
    if (!result || !runtimeStatus) {
      return;
    }

    setSavingReport(true);
    setSavedReportPath(null);
    setErrorMessage(null);

    const reportLines = [
      "Minimal Linux Disk Cloner Run Report",
      `Generated: ${new Date().toISOString()}`,
      `Mode: ${runtimeStatus.mode}`,
      `Source: ${source?.path ?? "Unknown"}`,
      `Target: ${target?.path ?? "Unknown"}`,
      `Verification requested: ${verifyAfterClone ? "yes" : "no"}`,
      `Verification completed: ${result.verify_completed ? "yes" : "no"}`,
      `Final phase: ${result.phase}`,
      `Success: ${result.success ? "yes" : "no"}`,
      `Bytes copied: ${result.bytes_copied}`,
      `Elapsed seconds: ${result.elapsed_secs}`,
      `Average speed: ${result.average_speed ?? 0}`,
      `Command: ${commandPreview || "Unavailable"}`,
      "",
      "Warnings at start:",
      ...(warnings.length === 0 ? ["None"] : warnings.map((warning) => `- [${warning.severity}] ${warning.message}`)),
      "",
      "Final message:",
      result.final_message,
      "",
      "Raw output:",
      ...(result.raw_output.length === 0 ? ["No output captured."] : result.raw_output),
    ];

    try {
      const path = await invoke<string>("save_run_report", {
        reportText: reportLines.join("\n"),
      });
      setSavedReportPath(path);
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setSavingReport(false);
    }
  }

  function resetFlow() {
    setPhase("setup");
    setProgress(null);
    setResult(null);
    setShowLogs(false);
    setStarting(false);
    setStopping(false);
    setSavingReport(false);
    setSavedReportPath(null);
    setShowStopModal(false);
    setErrorMessage(null);
    void refreshState();
  }

  const canClone =
    Boolean(source && target) &&
    sourcePath !== targetPath &&
    (runtimeStatus?.mode === "fake" || Boolean(runtimeStatus?.is_root)) &&
    Boolean(runtimeStatus?.has_dd) &&
    Boolean(runtimeStatus?.has_lsblk) &&
    !starting;

  return (
    <div
      className={clsx(
        "relative min-h-screen overflow-hidden bg-[#12161d] text-slate-100",
        !backgroundMotionEnabled && "app-motion-disabled",
      )}
    >
      <div className="app-background-base pointer-events-none absolute inset-0" />
      <div className="app-background-orb app-background-orb--warm pointer-events-none absolute inset-0" />
      <div className="app-background-orb app-background-orb--cool pointer-events-none absolute inset-0" />
      <div className="app-background-glow pointer-events-none absolute inset-0" />
      <div className="app-background-grid pointer-events-none absolute inset-0" />
      <div className="app-background-vignette pointer-events-none absolute inset-0" />
      <div className="relative mx-auto flex min-h-screen w-full max-w-7xl flex-col px-8 py-8">
        <header className="flex items-start justify-between">
          <div className="w-28" />
          <div className="flex items-center gap-3">
            <div className="flex size-12 items-center justify-center rounded-2xl border border-white/8 bg-white/6 shadow-[0_10px_30px_rgba(0,0,0,0.25)] backdrop-blur">
              <img src={logoUrl} alt="MLDC" className="size-8" />
            </div>
            <div className="text-center">
              <p className="text-[11px] font-semibold uppercase tracking-[0.28em] text-orange-300/80">
                Clone and verify physical drives
              </p>
              <h1 className="mt-1 text-3xl font-semibold tracking-tight text-white">
                Minimal Linux Disk Cloner
              </h1>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => setShowSettingsModal(true)}
              className="inline-flex items-center justify-center rounded-xl border border-white/8 bg-white/6 p-3 text-slate-100 shadow-[0_10px_30px_rgba(0,0,0,0.16)] transition hover:border-orange-300/25 hover:bg-white/10"
              aria-label="Open settings"
            >
              <HiCog6Tooth className="size-5" />
            </button>
            <button
              type="button"
              onClick={() => void refreshState()}
              className="inline-flex items-center gap-2 rounded-xl border border-white/8 bg-white/6 px-4 py-2 text-sm font-medium text-slate-100 shadow-[0_10px_30px_rgba(0,0,0,0.16)] transition hover:border-orange-300/25 hover:bg-white/10 disabled:opacity-60"
              disabled={refreshing}
            >
              <HiArrowPath className={clsx("size-4", refreshing && "animate-spin")} />
              Refresh
            </button>
          </div>
        </header>

        <main className="flex flex-1 items-center justify-center py-6">
          <AnimatePresence mode="wait">
            {phase === "setup" && (
              <motion.section
                key="setup"
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -12 }}
                transition={{ duration: 0.2, ease: "easeOut" }}
                className="w-full max-w-6xl"
              >
                <div className="mx-auto flex min-h-[640px] w-full flex-col">
                  <StatusStrip runtimeStatus={runtimeStatus} errorMessage={errorMessage} />

                  <div className="mt-16 grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_96px_minmax(0,1fr)_96px_minmax(0,1fr)]">
                    <StageCard
                      icon={<HiCircleStack className="size-7" />}
                      title="Source"
                      subtitle="Choose source"
                      accent="cyan"
                    >
                      <DeviceSelect
                        label="Source disk"
                        value={sourcePath}
                        onChange={setSourcePath}
                        devices={devices}
                        selectedDevice={source}
                      />
                      <DeviceSummary device={source} emptyLabel="Clone drive" />
                    </StageCard>

                    <Connector />

                    <StageCard
                      icon={<HiServerStack className="size-7" />}
                      title="Target"
                      subtitle="Select target"
                      accent="slate"
                    >
                      <DeviceSelect
                        label="Target disk"
                        value={targetPath}
                        onChange={setTargetPath}
                        devices={devices}
                        selectedDevice={target}
                      />
                      <DeviceSummary
                        device={target}
                        emptyLabel="Choose target"
                      />
                    </StageCard>

                    <Connector />

                    <StageCard
                      icon={<HiBolt className="size-7" />}
                      title="Flash"
                      subtitle="Start clone"
                      accent="emerald"
                    >
                      <button
                        type="button"
                        onClick={() => void beginClone()}
                        disabled={!canClone}
                        className={clsx(
                          "mt-3 inline-flex w-full items-center justify-center rounded-full border px-5 py-4 text-base font-semibold transition active:translate-y-px",
                          canClone
                            ? "border-orange-200/15 bg-[linear-gradient(180deg,#f59e0b,#ea580c)] text-[#1f232a] shadow-[0_16px_32px_rgba(249,115,22,0.28)] hover:brightness-105"
                            : "cursor-not-allowed border-white/6 bg-[linear-gradient(180deg,#313742,#272c35)] text-slate-500 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
                        )}
                      >
                        {starting ? "Starting…" : "Flash!"}
                      </button>
                      <div className="mt-4 min-h-12 text-sm text-slate-300">
                        <p
                          className={clsx(
                            "font-medium",
                            canClone ? "text-orange-100" : "text-slate-400",
                          )}
                        >
                          {source && target
                            ? `${source.path} → ${target.path}`
                            : "Pick both disks first"}
                        </p>
                      </div>
                      <label className="mt-3 flex items-center justify-center gap-3 text-sm text-slate-300">
                        <input
                          type="checkbox"
                          checked={verifyAfterClone}
                          onChange={(event) => setVerifyAfterClone(event.target.checked)}
                          className="size-4 rounded border-white/15 bg-[#2d3138] accent-orange-500"
                        />
                        Verify after clone
                      </label>
                    </StageCard>
                  </div>

                  <div className="mt-10 min-h-[140px]">
                    <WarningsPanel warnings={warnings} />
                  </div>
                </div>
              </motion.section>
            )}

            {phase === "progress" && progress && (
              <motion.section
                key="progress"
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -12 }}
                transition={{ duration: 0.2, ease: "easeOut" }}
                className="w-full max-w-4xl"
              >
                <div className="rounded-[2rem] bg-transparent p-8">
                  <div className="text-center">
                    <motion.div
                      initial={{ opacity: 0.7 }}
                      animate={{ opacity: [0.7, 1, 0.7] }}
                      transition={{ duration: 1.6, repeat: Number.POSITIVE_INFINITY }}
                      className="mx-auto flex size-18 items-center justify-center rounded-full border border-orange-300/15 bg-white/6 text-orange-300 shadow-[0_12px_36px_rgba(249,115,22,0.18)]"
                    >
                      <HiBolt className="size-8" />
                    </motion.div>
                    <h2 className="mt-6 text-4xl font-semibold tracking-tight text-white">
                      {progress.phase === "verify"
                        ? "Verification in progress"
                        : "Cloning in progress"}
                    </h2>
                    <p className="mt-3 text-base text-slate-300">
                      {progress.phase === "verify"
                        ? "Keep the app open until the verification pass completes."
                        : "Keep the app open until the write completes."}
                    </p>
                  </div>

                  <div className="mt-8 overflow-hidden rounded-[1.75rem] border border-white/7 bg-[linear-gradient(180deg,rgba(255,255,255,0.07),rgba(255,255,255,0.03))] p-6 shadow-[0_16px_40px_rgba(0,0,0,0.22)] backdrop-blur">
                    <div className="flex items-end justify-between gap-6">
                      <div>
                        <p className="text-sm uppercase tracking-[0.24em] text-slate-400">
                          {progress.phase === "verify" ? "Verifying" : "Cloning"}
                        </p>
                        <p className="mt-2 text-5xl font-semibold text-white">
                          {(progress.percent * 100).toFixed(1)}%
                        </p>
                      </div>
                      <div className="text-right text-sm text-slate-400">
                        <p>{formatBytes(progress.bytes_copied)} copied</p>
                        <p>{formatBytes(progress.total_bytes)} total</p>
                      </div>
                    </div>

                    <div className="mt-6 h-4 rounded-full bg-[#16191f] ring-1 ring-white/5">
                      <motion.div
                        className="h-4 rounded-full bg-[linear-gradient(90deg,#fb923c,#f97316,#f59e0b)] shadow-[0_0_24px_rgba(249,115,22,0.45)]"
                        initial={{ width: 0 }}
                        animate={{ width: `${Math.max(progress.percent * 100, 2)}%` }}
                        transition={{ duration: 0.35, ease: "easeOut" }}
                      />
                    </div>

                    <div className="mt-6 grid gap-4 sm:grid-cols-4">
                      <StatCard label="Speed" value={progress.speed_bytes_per_sec ? `${formatBytes(progress.speed_bytes_per_sec)}/s` : "Waiting"} />
                      <StatCard label="Elapsed" value={formatDuration(progress.elapsed_secs)} />
                      <StatCard label="ETA" value={progress.eta_secs ? formatDuration(progress.eta_secs) : "Unknown"} />
                      <StatCard
                        label="Mode"
                        value={progress.phase === "verify" ? "Full compare" : "dd status=progress"}
                      />
                    </div>
                  </div>

                  <DetailsPanel
                    title="Live log"
                    open={showLogs}
                    onToggle={() => setShowLogs((value) => !value)}
                    lines={progress.raw_output}
                    footer={commandPreview}
                  />

                  <div className="mt-8 flex justify-center">
                    <button
                      type="button"
                      onClick={() => setShowStopModal(true)}
                      disabled={stopping}
                      className="inline-flex items-center gap-2 rounded-full border border-rose-300/20 bg-rose-400/10 px-6 py-3 text-sm font-semibold text-rose-100 transition hover:bg-rose-400/16 disabled:cursor-not-allowed disabled:opacity-60"
                    >
                      <HiXCircle className="size-4" />
                      {stopping ? "Stopping…" : "Stop run"}
                    </button>
                  </div>
                </div>
              </motion.section>
            )}

            {phase === "result" && result && (
              <motion.section
                key="result"
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -12 }}
                transition={{ duration: 0.2, ease: "easeOut" }}
                className="w-full max-w-4xl"
              >
                <div className="rounded-[2rem] bg-transparent p-8">
                  <div className="text-center">
                    <div
                      className={clsx(
                        "mx-auto flex size-18 items-center justify-center rounded-full border bg-white/6 shadow-[0_12px_36px_rgba(0,0,0,0.18)]",
                        result.success
                          ? "border-emerald-400/20 text-emerald-300"
                          : "border-rose-400/20 text-rose-300",
                      )}
                    >
                      {result.success ? (
                        <HiCheckCircle className="size-8" />
                      ) : (
                        <HiXCircle className="size-8" />
                      )}
                    </div>
                    <h2 className="mt-6 text-4xl font-semibold tracking-tight text-white">
                      {result.success
                        ? result.verify_completed
                          ? "Clone verified"
                          : "Clone complete"
                        : result.phase === "verify"
                          ? "Verification failed"
                          : "Clone failed"}
                    </h2>
                    <p className="mt-3 text-base text-slate-300">
                      {result.final_message}
                    </p>
                  </div>

                  <div className="mt-8 grid gap-4 sm:grid-cols-3">
                    <StatCard label="Copied" value={formatBytes(result.bytes_copied)} />
                    <StatCard label="Elapsed" value={formatDuration(result.elapsed_secs)} />
                    <StatCard
                      label={result.phase === "verify" ? "Verify speed" : "Average speed"}
                      value={
                        result.average_speed
                          ? `${formatBytes(result.average_speed)}/s`
                          : "Unknown"
                      }
                    />
                  </div>

                  {result.verify_requested && (
                    <div className="mt-4 grid gap-4 sm:grid-cols-2">
                      <StatCard label="Verification" value={result.verify_completed ? "Passed" : "Failed"} />
                      <StatCard label="Final phase" value={result.phase === "verify" ? "Verification" : "Cloning"} />
                    </div>
                  )}

                  <DetailsPanel
                    title="Run details"
                    open={showLogs}
                    onToggle={() => setShowLogs((value) => !value)}
                    lines={result.raw_output}
                    footer={commandPreview}
                  />

                  <div className="mt-8 flex justify-center">
                    <div className="flex flex-wrap justify-center gap-3">
                      <button
                        type="button"
                        onClick={() => void saveRunReport()}
                        disabled={savingReport}
                        className="inline-flex items-center gap-2 rounded-full border border-orange-300/20 bg-orange-400/10 px-6 py-3 text-sm font-semibold text-orange-100 transition hover:bg-orange-400/16 disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        <HiDocumentArrowDown className="size-4" />
                        {savingReport ? "Saving…" : "Save report"}
                      </button>
                      <button
                        type="button"
                        onClick={resetFlow}
                        className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/8 px-6 py-3 text-sm font-semibold text-white transition hover:border-orange-300/30 hover:bg-white/12"
                      >
                        <HiArrowPath className="size-4" />
                        Start over
                      </button>
                    </div>
                  </div>

                  {savedReportPath && (
                    <p className="mt-4 text-center text-sm text-slate-300">
                      Report saved to {savedReportPath}
                    </p>
                  )}
                </div>
              </motion.section>
            )}
          </AnimatePresence>
        </main>

        <AnimatePresence>
          {showSettingsModal && (
            <SettingsModal
              appVersion={appVersion}
              runtimeStatus={runtimeStatus}
              defaultVerifyAfterClone={defaultVerifyAfterClone}
              backgroundMotionEnabled={backgroundMotionEnabled}
              onClose={() => setShowSettingsModal(false)}
              onDefaultVerifyChange={(value) => {
                setDefaultVerifyAfterClone(value);
                setVerifyAfterClone(value);
                if (typeof window !== "undefined") {
                  window.localStorage.setItem(VERIFY_DEFAULT_STORAGE_KEY, String(value));
                }
              }}
              onBackgroundMotionChange={(value) => {
                setBackgroundMotionEnabled(value);
                if (typeof window !== "undefined") {
                  window.localStorage.setItem(BACKGROUND_MOTION_STORAGE_KEY, String(value));
                }
              }}
            />
          )}
          {showStopModal && (
            <ConfirmModal
              title="Stop current run?"
              message="The current clone or verification process will be stopped. The target disk may be left incomplete."
              confirmLabel={stopping ? "Stopping…" : "Stop run"}
              cancelLabel="Keep running"
              onConfirm={() => void stopClone()}
              onCancel={() => {
                if (!stopping) {
                  setShowStopModal(false);
                }
              }}
              disabled={stopping}
            />
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

function ConfirmModal({
  title,
  message,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
  disabled,
}: {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  disabled: boolean;
}) {
  useEffect(() => {
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape" && !disabled) {
        onCancel();
      }
    }

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [disabled, onCancel]);

  if (typeof document === "undefined") {
    return null;
  }

  return createPortal(
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 px-6 backdrop-blur-sm"
    >
      <motion.div
        initial={{ opacity: 0, y: 12, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 12, scale: 0.98 }}
        transition={{ duration: 0.18, ease: "easeOut" }}
        className="w-full max-w-md rounded-[1.75rem] border border-white/10 bg-[#2a2f36] p-6 shadow-[0_24px_60px_rgba(0,0,0,0.42)]"
      >
        <div className="flex items-center gap-3">
          <div className="flex size-12 items-center justify-center rounded-full border border-rose-300/20 bg-rose-400/10 text-rose-200">
            <HiExclamationTriangle className="size-6" />
          </div>
          <div>
            <h3 className="text-xl font-semibold text-white">{title}</h3>
            <p className="mt-1 text-sm text-slate-300">{message}</p>
          </div>
        </div>

        <div className="mt-6 flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={disabled}
            className="rounded-full border border-white/10 bg-white/7 px-5 py-2.5 text-sm font-semibold text-slate-200 transition hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={disabled}
            className="rounded-full border border-rose-300/20 bg-rose-400/12 px-5 py-2.5 text-sm font-semibold text-rose-100 transition hover:bg-rose-400/18 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {confirmLabel}
          </button>
        </div>
      </motion.div>
    </motion.div>,
    document.body,
  );
}

function SettingsModal({
  appVersion,
  runtimeStatus,
  defaultVerifyAfterClone,
  backgroundMotionEnabled,
  onClose,
  onDefaultVerifyChange,
  onBackgroundMotionChange,
}: {
  appVersion: string;
  runtimeStatus: RuntimeStatus | null;
  defaultVerifyAfterClone: boolean;
  backgroundMotionEnabled: boolean;
  onClose: () => void;
  onDefaultVerifyChange: (value: boolean) => void;
  onBackgroundMotionChange: (value: boolean) => void;
}) {
  useEffect(() => {
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [onClose]);

  if (typeof document === "undefined") {
    return null;
  }

  return createPortal(
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 px-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <motion.div
        initial={{ opacity: 0, y: 12, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 12, scale: 0.98 }}
        transition={{ duration: 0.18, ease: "easeOut" }}
        className="w-full max-w-lg rounded-[1.75rem] border border-white/10 bg-[#2a2f36] p-6 shadow-[0_24px_60px_rgba(0,0,0,0.42)]"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.28em] text-orange-300/80">
              Preferences
            </p>
            <h3 className="mt-2 text-2xl font-semibold text-white">Settings</h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="inline-flex items-center justify-center rounded-xl border border-white/8 bg-white/6 p-2 text-slate-200 transition hover:bg-white/10"
            aria-label="Close settings"
          >
            <HiXCircle className="size-5" />
          </button>
        </div>

        <div className="mt-6 space-y-4">
          <SettingToggle
            title="Enable verification by default"
            description="Use the Verify after clone option as the default for new runs."
            checked={defaultVerifyAfterClone}
            onChange={onDefaultVerifyChange}
          />
          <SettingToggle
            title="Animate background"
            description="Keep the ambient background motion enabled."
            checked={backgroundMotionEnabled}
            onChange={onBackgroundMotionChange}
          />
        </div>

        <div className="mt-6 rounded-2xl border border-white/8 bg-white/5 px-4 py-4">
          <p className="text-xs font-semibold uppercase tracking-[0.24em] text-slate-400">
            App info
          </p>
          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <InfoRow label="Version" value={appVersion} />
            <InfoRow label="Mode" value={runtimeStatus?.mode ?? "unknown"} />
            <InfoRow
              label="Privileges"
              value={runtimeStatus?.mode === "fake" ? "simulation" : runtimeStatus?.is_root ? "root" : "user"}
            />
            <InfoRow
              label="Tools"
              value={
                runtimeStatus
                  ? `${runtimeStatus.has_lsblk ? "lsblk" : "no lsblk"} / ${runtimeStatus.has_dd ? "dd" : "no dd"}`
                  : "checking"
              }
            />
          </div>
        </div>
      </motion.div>
    </motion.div>,
    document.body,
  );
}

function SettingToggle({
  title,
  description,
  checked,
  onChange,
}: {
  title: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="flex items-start justify-between gap-4 rounded-2xl border border-white/8 bg-white/5 px-4 py-4">
      <div>
        <p className="font-medium text-white">{title}</p>
        <p className="mt-1 text-sm text-slate-400">{description}</p>
      </div>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="mt-1 size-4 rounded border-white/15 bg-[#2d3138] accent-orange-500"
      />
    </label>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-white/6 bg-black/10 px-3 py-3">
      <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-slate-500">
        {label}
      </p>
      <p className="mt-2 text-sm font-medium text-slate-200">{value}</p>
    </div>
  );
}

function StatusStrip({
  runtimeStatus,
  errorMessage,
}: {
  runtimeStatus: RuntimeStatus | null;
  errorMessage: string | null;
}) {
  const messages = [
    ...(runtimeStatus?.errors ?? []),
    ...(errorMessage ? [errorMessage] : []),
  ];

  if (messages.length === 0 && runtimeStatus) {
    if (runtimeStatus.mode === "fake") {
      return (
        <div className="mt-10 flex flex-wrap justify-center gap-3 text-sm">
          <StatusChip label="Simulation mode" ok />
          <StatusChip label="No real disks touched" ok />
        </div>
      );
    }

    return (
      <div className="mt-10 flex flex-wrap justify-center gap-3 text-sm">
        <StatusChip label={runtimeStatus.is_root ? "Running as root" : "Needs root"} ok={runtimeStatus.is_root} />
        <StatusChip label="lsblk ready" ok={runtimeStatus.has_lsblk} />
        <StatusChip label="dd ready" ok={runtimeStatus.has_dd} />
      </div>
    );
  }

  return (
    <div className="mx-auto mt-10 max-w-3xl rounded-2xl border border-rose-300/12 bg-[linear-gradient(180deg,rgba(255,255,255,0.05),rgba(255,255,255,0.025))] px-5 py-4 text-sm text-rose-100 shadow-[0_10px_30px_rgba(0,0,0,0.14)]">
      <div className="flex items-center justify-center gap-2 text-rose-200">
        <HiExclamationTriangle className="size-5" />
        <span className="font-semibold">Runtime checks</span>
      </div>
      <ul className="mt-3 space-y-2 text-center text-rose-100/90">
        {messages.map((message) => (
          <li key={message}>• {message}</li>
        ))}
      </ul>
    </div>
  );
}

function StatusChip({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div
      className={clsx(
        "rounded-full border px-4 py-2 text-sm font-medium",
        ok
          ? "border-orange-300/10 bg-[#2d3138] text-slate-100 shadow-[0_8px_24px_rgba(0,0,0,0.14)]"
          : "border-rose-300/12 bg-[#2d3138] text-rose-200 shadow-[0_8px_24px_rgba(0,0,0,0.14)]",
      )}
    >
      {label}
    </div>
  );
}

function StageCard({
  icon,
  title,
  subtitle,
  accent,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  accent: "cyan" | "slate" | "emerald";
  children: React.ReactNode;
}) {
  const accentClasses =
    accent === "cyan"
      ? "text-orange-300"
      : accent === "emerald"
        ? "text-slate-200"
        : "text-slate-300";

  return (
    <div className={clsx(stageBase, accentClasses)}>
      <div className="flex flex-col items-center text-center">
        <motion.div
          whileHover={{ scale: 1.04, y: -1 }}
          transition={{ duration: 0.18, ease: "easeOut" }}
          className="flex size-16 items-center justify-center rounded-full border border-white/8 bg-[linear-gradient(180deg,#fff7ed,#fde68a)] text-[#3b2b1d] shadow-[0_10px_30px_rgba(0,0,0,0.18)]"
        >
          {icon}
        </motion.div>
        <p className="mt-4 text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-400">
          {subtitle}
        </p>
        <h3 className="mt-2 text-2xl font-medium tracking-tight text-white">{title}</h3>
      </div>
      <div className="mt-8 w-full">{children}</div>
    </div>
  );
}

function Connector() {
  return (
    <div className="hidden items-center justify-center lg:flex">
      <div className="h-px w-full bg-[linear-gradient(90deg,rgba(255,255,255,0.02),rgba(251,146,60,0.35),rgba(255,255,255,0.02))]" />
    </div>
  );
}

function DeviceSelect({
  label,
  value,
  onChange,
  devices,
  selectedDevice,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  devices: DiskDevice[];
  selectedDevice: DiskDevice | null;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    function handlePointerDown(event: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }

    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative block">
      <span className="mb-3 block text-sm font-medium text-slate-300">{label}</span>
      <motion.button
        type="button"
        onClick={() => setOpen((current) => !current)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            setOpen((current) => !current);
          }
        }}
        className="flex w-full items-center justify-between rounded-full border border-white/8 bg-[#2d3138] px-5 py-4 text-left shadow-[0_10px_28px_rgba(0,0,0,0.16)] transition hover:border-orange-300/20 hover:bg-[#343942]"
        aria-expanded={open}
        aria-haspopup="listbox"
        whileHover={{ y: -1 }}
        whileTap={{ scale: 0.995 }}
      >
        <span className="truncate text-base font-medium text-slate-200">
          {selectedDevice ? selectDeviceLabel(selectedDevice) : label}
        </span>
        <HiChevronDown
          className={clsx("size-5 text-slate-400 transition", open && "rotate-180")}
        />
      </motion.button>
      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 8 }}
            transition={{ duration: 0.14, ease: "easeOut" }}
            className="absolute left-0 right-0 z-20 mt-3 overflow-hidden rounded-[1.5rem] border border-white/8 bg-[#2a2f36] shadow-[0_24px_50px_rgba(15,23,42,0.45)] backdrop-blur"
          >
            <div className="max-h-80 overflow-auto p-2" role="listbox" aria-label={label}>
              {devices.map((device) => {
                const selected = device.path === value;
                return (
                  <button
                    key={device.path}
                    type="button"
                    onClick={() => {
                      onChange(device.path);
                      setOpen(false);
                    }}
                    className={clsx(
                      "mb-2 flex w-full items-start justify-between rounded-2xl px-4 py-3 text-left last:mb-0 transition",
                      selected
                        ? "border border-orange-300/16 bg-orange-400/10"
                        : "bg-transparent hover:bg-white/6",
                    )}
                  >
                    <div className="min-w-0">
                      <p className="truncate font-medium text-white">{device.path}</p>
                      <p className="mt-1 truncate text-sm text-slate-400">
                        {device.model ?? "Unknown model"}
                      </p>
                      <div className="mt-2 flex flex-wrap gap-2">
                        <span className="rounded-full bg-white/8 px-2.5 py-1 text-[11px] font-semibold text-slate-300">
                          {device.size_human}
                        </span>
                        {device.transport && (
                          <span className="rounded-full border border-white/8 px-2.5 py-1 text-[11px] text-slate-300">
                            {device.transport}
                          </span>
                        )}
                        {device.removable && (
                          <span className="rounded-full border border-amber-400/20 bg-amber-400/10 px-2.5 py-1 text-[11px] text-amber-200">
                            removable
                          </span>
                        )}
                        {device.mounted_partitions.length > 0 && (
                          <span className="rounded-full border border-rose-400/20 bg-rose-400/10 px-2.5 py-1 text-[11px] text-rose-200">
                            {device.mounted_partitions.length} mounted
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="ml-4 mt-1 flex size-6 items-center justify-center">
                      {selected && <HiCheck className="size-5 text-orange-300" />}
                    </div>
                  </button>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function DeviceSummary({
  device,
  emptyLabel,
}: {
  device: DiskDevice | null;
  emptyLabel: string;
}) {
  if (!device) {
    return <div className="mt-5 min-h-10 text-sm text-slate-400">{emptyLabel}</div>;
  }

  return (
    <div className="mt-5 min-h-10 text-sm text-slate-300">
      <p className="font-medium text-white">{device.path}</p>
      <p className="mt-1 text-slate-400">{device.model ?? device.size_human}</p>
      <div className="mt-3 flex flex-wrap justify-center gap-2">
        <span className="rounded-full bg-white/8 px-3 py-1 text-xs font-semibold text-slate-300">
          {device.size_human}
        </span>
        {device.transport && (
          <span className="rounded-full border border-white/8 px-2.5 py-1 text-xs text-slate-300">
            {device.transport}
          </span>
        )}
        {device.removable && (
          <span className="rounded-full border border-amber-400/20 bg-amber-400/10 px-2.5 py-1 text-xs text-amber-200">
            removable
          </span>
        )}
        {device.mounted_partitions.length > 0 && (
          <span className="rounded-full border border-rose-400/20 bg-rose-400/10 px-2.5 py-1 text-xs text-rose-200">
            {device.mounted_partitions.length} mounted
          </span>
        )}
      </div>
    </div>
  );
}

function WarningsPanel({ warnings }: { warnings: WarningItem[] }) {
  if (warnings.length === 0) {
    return (
      <div
        aria-hidden="true"
        className="mx-auto max-w-4xl rounded-2xl border border-transparent bg-transparent px-5 py-4 opacity-0"
      >
        <div className="min-h-[108px]" />
      </div>
    );
  }

  return (
    <div className="mx-auto mt-10 max-w-4xl rounded-2xl border border-white/8 bg-[linear-gradient(180deg,rgba(255,255,255,0.055),rgba(255,255,255,0.03))] px-5 py-4 shadow-[0_10px_30px_rgba(0,0,0,0.14)]">
      <div className="flex items-center justify-center gap-2 font-semibold text-amber-200">
        <HiInformationCircle className="size-5" />
        Review warnings
      </div>
      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        {warnings.map((warning) => (
          <div
            key={warning.code}
            className={clsx(
              "rounded-2xl border px-4 py-3 text-sm",
              warning.severity === "Danger"
                ? "border-rose-400/25 bg-rose-400/10 text-rose-100"
                : warning.severity === "Warning"
                  ? "border-amber-400/20 bg-amber-400/10 text-amber-100"
                  : "border-cyan-400/20 bg-cyan-400/10 text-cyan-100",
            )}
          >
            {warning.message}
          </div>
        ))}
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-[1.25rem] border border-white/7 bg-[linear-gradient(180deg,rgba(255,255,255,0.06),rgba(255,255,255,0.03))] px-4 py-4 shadow-[0_10px_24px_rgba(0,0,0,0.14)]">
      <p className="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">
        {label}
      </p>
      <p className="mt-2 text-lg font-semibold text-white">{value}</p>
    </div>
  );
}

function DetailsPanel({
  title,
  open,
  onToggle,
  lines,
  footer,
}: {
  title: string;
  open: boolean;
  onToggle: () => void;
  lines: string[];
  footer: string;
}) {
  return (
    <div className="mt-8 rounded-[1.5rem] border border-white/7 bg-[linear-gradient(180deg,rgba(255,255,255,0.06),rgba(255,255,255,0.03))] shadow-[0_10px_24px_rgba(0,0,0,0.14)]">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center justify-between px-5 py-4 text-left"
      >
        <span className="text-sm font-semibold uppercase tracking-[0.24em] text-slate-400">
          {title}
        </span>
        <span className="text-sm text-slate-300">{open ? "Hide" : "Show"}</span>
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            className="overflow-hidden"
          >
            <div className="border-t border-white/10 px-5 py-4">
              {footer && (
                <div className="mb-4 rounded-2xl border border-white/8 bg-white/5 px-4 py-3 font-mono text-xs text-slate-300">
                  {footer}
                </div>
              )}
              <div className="max-h-60 overflow-auto rounded-2xl bg-black/30 p-4 font-mono text-xs leading-6 text-slate-300">
                {lines.length === 0 ? (
                  <p className="text-slate-500">No output yet.</p>
                ) : (
                  lines.map((line, index) => <p key={`${line}-${index}`}>{line}</p>)
                )}
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function formatBytes(bytes: number) {
  const units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
  if (bytes <= 0) return "0 B";

  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return unitIndex === 0 ? `${value} ${units[unitIndex]}` : `${value.toFixed(1)} ${units[unitIndex]}`;
}

function selectDeviceLabel(device: DiskDevice) {
  return `${device.path} · ${device.size_human}`;
}

function formatDuration(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);

  if (hours > 0) {
    return `${hours.toString().padStart(2, "0")}:${minutes
      .toString()
      .padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${minutes.toString().padStart(2, "0")}:${seconds
    .toString()
    .padStart(2, "0")}`;
}
