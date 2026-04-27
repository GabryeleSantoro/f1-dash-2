import { Suspense } from "react";

import Archive from "@/components/archive/Archive";

type Props = {
	searchParams: Promise<{ year?: string }>;
};

export default async function ArchivePage({ searchParams }: Props) {
	const { year } = await searchParams;
	const yearNum = year ? Number.parseInt(year, 10) : undefined;
	const validYear = yearNum && !Number.isNaN(yearNum) ? yearNum : undefined;

	return (
		<div>
			<div className="my-4">
				<h1 className="text-3xl">Archive</h1>
				<p className="text-zinc-500">Replay past sessions from F1&apos;s static archive</p>
			</div>

			<Suspense fallback={<ArchiveLoading />}>
				<Archive year={validYear} />
			</Suspense>
		</div>
	);
}

const ArchiveLoading = () => {
	return (
		<div className="mb-20 grid grid-cols-1 gap-8 md:grid-cols-2">
			{Array.from({ length: 6 }).map((_, i) => (
				<div key={`meeting.${i}`} className="flex flex-col gap-2">
					<div className="h-12 w-full animate-pulse rounded-md bg-zinc-800" />
					<div className="h-10 w-full animate-pulse rounded-md bg-zinc-800" />
					<div className="h-10 w-full animate-pulse rounded-md bg-zinc-800" />
				</div>
			))}
		</div>
	);
};
