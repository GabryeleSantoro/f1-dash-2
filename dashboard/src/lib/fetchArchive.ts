import { connection } from "next/server";

import { env } from "@/env";

export type ArchiveSession = {
	key: number;
	name: string;
	type: string;
	path: string;
	startDate: string;
	endDate: string;
	gmtOffset: string;
};

export type ArchiveMeeting = {
	key: number;
	name: string;
	officialName: string;
	location: string;
	countryCode: string;
	countryName: string;
	sessions: ArchiveSession[];
};

export const getArchive = async (year?: number): Promise<ArchiveMeeting[] | null> => {
	await connection();

	try {
		const url = year ? `${env.API_URL}/api/archive?year=${year}` : `${env.API_URL}/api/archive`;
		const req = await fetch(url, { cache: "no-store" });
		if (!req.ok) return null;
		const data: ArchiveMeeting[] = await req.json();
		return data;
	} catch (e) {
		console.error("error fetching archive", e);
		return null;
	}
};
