import assert from 'node:assert/strict';

// Check the derivatives of the actual SVG segments, including joins hidden by
// node circles. Rounded stroke caps cannot make a cusp pass these checks.
export async function assertSmoothPaths(page) {
  const results = await page.$$eval('[role=tree] path', paths => paths.map(path => {
    const tokens = path.getAttribute('d').trim().split(/\s+/);
    const point = () => [Number(tokens.shift()), Number(tokens.shift())];
    if (tokens.shift() !== 'M') throw new Error('A route must start with M');
    let start = point();
    const segments = [];
    let curves = 0;
    while (tokens.length) {
      const command = tokens.shift();
      let first, last, end;
      if (command === 'C') {
        first = point(); last = point(); end = point(); curves += 1;
      } else if (command === 'L') {
        end = point();
        first = last = [(start[0] + end[0]) / 2, (start[1] + end[1]) / 2];
      } else throw new Error(`Unexpected route command: ${command}`);
      segments.push({ start, first, last, end });
      start = end;
    }
    const subtract = (to, from) => [to[0] - from[0], to[1] - from[1]];
    const length = ([x, y]) => Math.hypot(x, y);
    let previous;
    const issues = [];
    for (const segment of segments) {
      const enter = subtract(segment.first, segment.start);
      const leave = subtract(segment.end, segment.last);
      for (const tangent of [enter, leave]) {
        if (Math.abs(tangent[0]) > 0.001) issues.push('nonvertical tangent');
        if (length(tangent) < 0.001) issues.push('zero derivative');
      }
      if (previous) {
        const cross = previous[0] * enter[1] - previous[1] * enter[0];
        const dot = previous[0] * enter[0] + previous[1] * enter[1];
        if (Math.abs(cross) > 0.001 || dot <= 0) issues.push('sharp join or cusp');
      }
      previous = leave;
    }
    if (!segments.length) issues.push('empty path');
    return { key: path.dataset.connectionKey, kind: path.dataset.kind, curves, issues };
  }));
  assert.ok(results.length > 0, 'the graph has connections to inspect');
  assert.ok(results.some(path => path.curves > 0), 'the sample exercises curved branches');
  for (const path of results) assert.deepEqual(path.issues, [], `smooth ${path.kind} route ${path.key}`);
  return results.length;
}
