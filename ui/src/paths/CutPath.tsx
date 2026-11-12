import { splitPathTail } from './pathTail';
import './CutPath.css';

/**
 * `path` on one line, cut in the middle so its last folder stays readable as
 * long as possible. The tooltip is `title`, or the path itself.
 */
export function CutPath({
  path,
  title = path,
  className,
}: {
  path: string;
  title?: string;
  className?: string;
}) {
  const { head, tail } = splitPathTail(path);
  return (
    <span className={className ? `cut-path ${className}` : 'cut-path'} title={title}>
      <span className="cut-path-head">{head}</span>
      <span className="cut-path-tail">{tail}</span>
    </span>
  );
}
