import type { RejectDestination } from './destination';

/** Outline icon of a folder or a trash can, in the current text colour. */
export function DestinationIcon({ destination }: { destination: RejectDestination }) {
  return (
    <svg
      className="reject-destination-icon"
      viewBox="0 0 24 24"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      data-icon={destination}
    >
      {destination === 'trash' ? (
        <path d="M4 7h16M10 11v6M14 11v6M5 7l1 13h12l1-13M9 7V4h6v3" />
      ) : (
        <path d="M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" />
      )}
    </svg>
  );
}
