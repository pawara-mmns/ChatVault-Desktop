import { Fragment } from 'react';
/** Map NFC/case-normalized grapheme matches back onto original text offsets. */
export function Highlight({
  text,
  query = '',
}: {
  text: string;
  query?: string;
}) {
  if (!query.trim()) return <>{text}</>;
  const needle = query.trim().normalize('NFC').toLowerCase().normalize('NFC');
  const starts: number[] = [];
  const ends: number[] = [];
  let folded = '';
  for (const segment of new Intl.Segmenter(undefined, {
    granularity: 'grapheme',
  }).segment(text)) {
    const normalized = segment.segment
      .normalize('NFC')
      .toLowerCase()
      .normalize('NFC');
    folded += normalized;
    for (let i = 0; i < normalized.length; i++) {
      starts.push(segment.index);
      ends.push(segment.index + segment.segment.length);
    }
  }
  const ranges: { start: number; end: number }[] = [];
  let from = 0;
  while (ranges.length < 200) {
    const found = folded.indexOf(needle, from);
    if (found < 0) break;
    const start = starts[found];
    const end = ends[found + needle.length - 1];
    if (start !== undefined && end !== undefined) {
      const previous = ranges.at(-1);
      if (previous && start <= previous.end) previous.end = end;
      else ranges.push({ start, end });
    }
    from = found + needle.length;
  }
  let offset = 0;
  const parts = ranges.map((range, index) => {
    const before = text.slice(offset, range.start);
    offset = range.end;
    return (
      <Fragment key={index}>
        {before}
        <mark>{text.slice(range.start, range.end)}</mark>
      </Fragment>
    );
  });
  return (
    <>
      {parts}
      {text.slice(offset)}
    </>
  );
}
