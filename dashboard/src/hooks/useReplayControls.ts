"use client";

import { useCallback } from "react";

import { env } from "@/env";

export type ReplaySpeed = 0.5 | 1 | 2 | 4;

export const useReplayControls = () => {
	const stop = useCallback(async (): Promise<boolean> => {
		try {
			const res = await fetch(`${env.NEXT_PUBLIC_LIVE_URL}/api/replay/stop`, { method: "POST" });
			return res.ok;
		} catch (e) {
			console.error("replay stop error", e);
			return false;
		}
	}, []);

	const setSpeed = useCallback(async (path: string, speed: ReplaySpeed): Promise<boolean> => {
		try {
			const res = await fetch(`${env.NEXT_PUBLIC_LIVE_URL}/api/replay/start`, {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ path, speed }),
			});
			return res.ok;
		} catch (e) {
			console.error("replay setSpeed error", e);
			return false;
		}
	}, []);

	return { stop, setSpeed };
};
