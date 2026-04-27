import Meeting from "@/components/archive/Meeting";
import { getArchive } from "@/lib/fetchArchive";

type Props = {
	year?: number;
};

export default async function Archive({ year }: Props) {
	const meetings = await getArchive(year);

	if (!meetings) {
		return (
			<div className="flex h-44 flex-col items-center justify-center">
				<p>Archive not found</p>
			</div>
		);
	}

	if (meetings.length === 0) {
		return (
			<div className="flex h-44 flex-col items-center justify-center">
				<p className="text-zinc-500">No meetings available for this year</p>
			</div>
		);
	}

	return (
		<div className="mb-20 grid grid-cols-1 gap-8 md:grid-cols-2">
			{meetings.map((meeting) => (
				<Meeting meeting={meeting} key={`meeting.${meeting.key}`} />
			))}
		</div>
	);
}
