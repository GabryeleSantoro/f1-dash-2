import { utc } from "moment";

import Flag from "@/components/Flag";
import WatchButton from "@/components/archive/WatchButton";
import { formatDayRange, formatMonth } from "@/lib/dateFormatter";
import type { ArchiveMeeting } from "@/lib/fetchArchive";

type Props = {
	meeting: ArchiveMeeting;
};

export default function Meeting({ meeting }: Props) {
	const sessions = meeting.sessions;
	const start = sessions[0]?.startDate ?? "";
	const end = sessions[sessions.length - 1]?.endDate ?? "";

	return (
		<div>
			<div className="flex items-center justify-between border-b border-zinc-800 pb-2">
				<div className="flex items-center gap-2">
					<Flag countryCode={meeting.countryCode} className="h-8 w-11" />
					<div className="flex flex-col leading-tight">
						<p className="text-2xl">{meeting.countryName}</p>
						<p className="text-xs text-zinc-500">{meeting.location}</p>
					</div>
				</div>

				{start && end && (
					<div className="flex gap-1">
						<p className="text-xl">{formatMonth(start, end)}</p>
						<p className="text-zinc-500">{formatDayRange(start, end)}</p>
					</div>
				)}
			</div>

			<div className="flex flex-col gap-2 pt-2">
				{sessions.map((session) => (
					<div
						key={`session.${session.key}`}
						className="flex items-center justify-between rounded-md bg-zinc-900 px-3 py-2"
					>
						<div className="flex flex-col leading-tight">
							<p className="text-base">{session.name}</p>
							<p className="text-xs text-zinc-500">{utc(session.startDate).local().format("ddd D MMM HH:mm")}</p>
						</div>

						<div className="flex items-center gap-2">
							<span className="rounded-full bg-zinc-800 px-2 py-0.5 text-xs text-zinc-300 uppercase">
								{session.type}
							</span>
							<WatchButton path={session.path} />
						</div>
					</div>
				))}
			</div>
		</div>
	);
}
