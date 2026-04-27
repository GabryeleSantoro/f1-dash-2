"use client";

import { useEffect, useState } from "react";

import { env } from "@/env";

export type ReplayStatus = {
	mode: "live" | "archive";
	path?: string;
	speed?: number;
	positionMs?: number;
	totalMs?: number;
};

export type ReplayStatusInfo = {
	isReplay: boolean;
	path?: string;
	positionMs?: number;
	totalMs?: number;
	speed?: number;
	ended: boolean;
};

const POLL_MS = 500;

const DEFAULT: ReplayStatusInfo = {
	isReplay: false,
	ended: false,
};

export const useReplayStatus = (): ReplayStatusInfo => {
	const [info, setInfo] = useState<ReplayStatusInfo>(DEFAULT);

	useEffect(() => {
		let cancelled = false;
		let timer: ReturnType<typeof setTimeout> | null = null;

		const tick = async () => {
			try {
				const res = await fetch(`${env.NEXT_PUBLIC_LIVE_URL}/api/replay/status`, { cache: "no-store" });
				if (!res.ok) throw new Error(`status ${res.status}`);
				const data: ReplayStatus = await res.json();
				if (cancelled) return;

				if (data.mode === "archive") {
					const positionMs = data.positionMs ?? 0;
					const totalMs = data.totalMs ?? 0;
					const ended = totalMs > 0 && positionMs >= totalMs;
					setInfo({
						isReplay: true,
						path: data.path,
						positionMs,
						totalMs,
						speed: data.speed,
						ended,
					});
				} else {
					setInfo(DEFAULT);
				}
			} catch {
				if (!cancelled) setInfo(DEFAULT);
			} finally {
				if (!cancelled) timer = setTimeout(tick, POLL_MS);
			}
		};

		tick();

		return () => {
			cancelled = true;
			if (timer) clearTimeout(timer);
		};
	}, []);

	return info;
};
