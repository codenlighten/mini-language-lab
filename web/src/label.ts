/**
 * Splits a step's label into at most `maxLines` lines of at most `width` characters, breaking at spaces.
 * Anything that still does not fit ends in an ellipsis. Lengths count code points, so an emoji is never cut in half.
 */
export function wrapLabel(text: string, width: number, maxLines: number): string[] {
  const words = text.split(/\s+/).filter(Boolean);
  const length = (s: string) => [...s].length;
  const lines: string[] = [];
  let next = 0;
  while (next < words.length && lines.length < maxLines) {
    let line = words[next++] as string;
    while (next < words.length && length(`${line} ${words[next]}`) <= width) line += ` ${words[next++]}`;
    lines.push(line);
  }
  const unfinished = next < words.length;
  return lines.map((line, i) => {
    const cut = length(line) > width || (unfinished && i === lines.length - 1);
    return cut ? `${[...line].slice(0, width - 1).join("")}…` : line;
  });
}
