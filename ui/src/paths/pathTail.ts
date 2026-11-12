/**
 * `path` split before its last folder or file name, so the start can be cut
 * with an ellipsis while the name stays whole: `C:\Zdjęcia\Wakacje` →
 * `{ head: 'C:\Zdjęcia\', tail: 'Wakacje' }`. Accepts `/` and `\`. A path
 * without a name after its last separator (`/`, `C:\`) is all head.
 */
export function splitPathTail(path: string): { head: string; tail: string } {
  const cut = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1;
  return { head: path.slice(0, cut), tail: path.slice(cut) };
}
