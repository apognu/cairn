import { useStalePages } from "./queries";
import * as ui from "./ui";

/** A review date as people write it: 15 Jan 2027. */
export function reviewDate(date: string) {
  // A calendar date, not an instant: read in UTC so no time zone moves it to the day before.
  return new Date(`${date}T00:00:00Z`).toLocaleDateString("en-GB", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  });
}

/** The pages whose review date has come, with who looks after them. */
export function ReviewSettings({ onOpenPage }: { onOpenPage: (id: string) => void }) {
  const stale = useStalePages();

  return (
    <div className="max-w-205">
      <h1 className={ui.settingsTitle}>Pages to review</h1>
      <p className="text-muted">
        Pages whose review date has come, the most overdue first. Set a page's owner and review date in its page
        settings.
      </p>

      <table className={ui.table} aria-label="Pages to review">
        <thead>
          <tr>
            <th scope="col" className={ui.th}>
              Page
            </th>
            <th scope="col" className={ui.th}>
              Owner
            </th>
            <th scope="col" className={ui.th}>
              Review by
            </th>
          </tr>
        </thead>
        <tbody>
          {stale.data?.length === 0 && (
            <tr>
              <td colSpan={3} className={`${ui.td} text-muted text-help`}>
                No page is due for a review.
              </td>
            </tr>
          )}
          {stale.data?.map((s) => (
            <tr key={s.page_id}>
              <td className={ui.td}>
                <button
                  type="button"
                  className={`${ui.linkButton} p-0 text-left`}
                  onClick={() => onOpenPage(s.page_id)}
                >
                  {s.title}
                </button>
              </td>
              <td className={ui.td}>{s.owner?.name ?? <span className="text-muted">Nobody</span>}</td>
              <td className={ui.td}>{reviewDate(s.review_by)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
