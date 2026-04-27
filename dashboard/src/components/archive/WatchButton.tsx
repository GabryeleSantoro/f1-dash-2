"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

import Button from "@/components/ui/Button";
import { env } from "@/env";

type Props = {
	path: string;
};

export default function WatchButton({ path }: Props) {
	const router = useRouter();
	const [loading, setLoading] = useState(false);

	const onClick = async () => {
		if (loading) return;
		setLoading(true);
		try {
			const res = await fetch(`${env.NEXT_PUBLIC_LIVE_URL}/api/replay/start`, {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ path, speed: 1 }),
			});
			if (!res.ok) {
				console.error("replay start failed", res.status);
				setLoading(false);
				return;
			}
			router.push("/dashboard");
		} catch (e) {
			console.error("replay start error", e);
			setLoading(false);
		}
	};

	return (
		<Button onClick={onClick} className="!bg-indigo-600 px-4 text-sm">
			{loading ? "Starting…" : "Watch"}
		</Button>
	);
}
