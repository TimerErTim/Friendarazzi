export function newestCapturesFirst<T extends { capturedAt: string }>(
  captures: readonly T[],
): T[] {
  return [...captures].sort(
    (left, right) => Date.parse(right.capturedAt) - Date.parse(left.capturedAt),
  );
}
