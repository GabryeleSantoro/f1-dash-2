"use client";

import { useRouter } from "next/navigation";
import { useCallback, useRef } from "react";

import { useReplayControls, type ReplaySpeed } from "@/hooks/useReplayControls";

import SegmentedControls from "@/components/ui/SegmentedControls";
import Button from "@/components/ui/Button";

const SPEEDS: { label: string; value: ReplaySpeed }[] = [
	{ label: "0.5×", value: 0.5 },
	{ label: "1×", value: 1 },
	{ label: "2×", value: 2 },
	{ label: "4×", value: 4 },
];

const formatTime = (ms: number): string => {
	const totalSec = Math.max(0, Math.floor(ms / 1000));
	const h = Math.floor(totalSec / 3600);
	const m = Math.floor((totalSec % 3600) / 60);
	const s = totalSec % 60;
	const pad = (n: number) => n.toString().padStart(2, "0");
	return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
};

const sessionNameFromPath = (path?: string): string => {
	if (!path) return "Replay";
	const parts = path.split("/").filter(Boolean);
	const last = parts[parts.length - 1] ?? "Replay";
	return last.replace(/^\d{4}-\d{2}-\d{2}_/, "").replace(/_/g, " ");
};

type Props = {
	path?: string;
	positionMs: number;
	totalMs: number;
	speed?: number;
};

export default function ReplayBar({ path, positionMs, totalMs, speed }: Props) {
	const router = useRouter();
	const { stop, setSpeed, seek } = useReplayControls();
	const barRef = useRef<HTMLDivElement | null>(null);

	const percent = totalMs > 0 ? Math.min(100, (positionMs / totalMs) * 100) : 0;
	const currentSpeed: ReplaySpeed = (speed ?? 1) as ReplaySpeed;

	const handleStop = useCallback(async () => {
		await stop();
		router.push("/");
	}, [stop, router]);

	const handleSpeed = useCallback(
		async (val: ReplaySpeed) => {
			if (!path) return;
			await setSpeed(path, val, positionMs);
		},
		[path, setSpeed, positionMs],
	);

	const handleSeek = useCallback(
		async (e: React.MouseEvent<HTMLDivElement>) => {
			if (totalMs <= 0 || !barRef.current) return;
			const rect = barRef.current.getBoundingClientRect();
			const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
			await seek(ratio * totalMs);
		},
		[seek, totalMs],
	);

	return (
		<div className="flex items-center gap-3">
			<span className="hidden max-w-[200px] truncate text-sm font-medium text-white sm:inline-block">
				{sessionNameFromPath(path)}
			</span>

			<div className="flex items-center gap-2">
				<span className="font-mono text-xs text-zinc-400 tabular-nums">{formatTime(positionMs)}</span>

				<div
					ref={barRef}
					onClick={handleSeek}
					title="Click to seek"
					className="h-1.5 w-32 cursor-pointer overflow-hidden rounded-full bg-zinc-700 transition-[height] hover:h-2 sm:w-40"
				>
					<div
						className="h-full bg-emerald-500 transition-[width] duration-500 ease-linear"
						style={{ width: `${percent}%` }}
					/>
				</div>

				<span className="font-mono text-xs text-zinc-400 tabular-nums">{formatTime(totalMs)}</span>
			</div>

			<SegmentedControls id="replay-speed" options={SPEEDS} selected={currentSpeed} onSelect={(v) => handleSpeed(v)} />

			<Button onClick={handleStop} className="bg-red-700!">
				Stop
			</Button>
		</div>
	);
}
