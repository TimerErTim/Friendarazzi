import { VisibilityMode } from "@/api/friendarazzi-api";

export type FlushPendingSaveFn = () => Promise<void>;

export function visibilityLabel(mode: VisibilityMode): string {
  switch (mode) {
    case VisibilityMode.Undercover:
      return "Undercover (verdeckt bis Bingo)";
    case VisibilityMode.LiveFeed:
      return "Live-Feed (sofort sichtbar)";
    case VisibilityMode.BlurredTeaser:
      return "Blurred Teaser (verpixelt)";
  }
}
