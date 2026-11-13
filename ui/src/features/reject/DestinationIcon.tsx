import type { RejectDestination } from './destination';

/** Outline icon of an arrow into a folder or of a trash can, in the current text colour. */
export function DestinationIcon({ destination }: { destination: RejectDestination }) {
  return (
    <svg
      className="reject-destination-icon"
      viewBox="0 0 24 24"
      width="20"
      height="20"
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
        // The arrow crossing into the folder says the photo is moved there, not just filed.
        <>
          <path d="M8 8.5V5a1 1 0 0 1 1-1h4.5l2 2H22a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1v-3.5" />
          <path d="M1 12h15M12 8l4 4-4 4" />
        </>
      )}
    </svg>
  );
}
