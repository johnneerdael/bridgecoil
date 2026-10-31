export type Level = "recommended" | "advanced" | "expert" | "unsafe";
export type Group =
  | "installed"
  | "recommended"
  | "advanced"
  | "notRated"
  | "expert"
  | "unsafe";
export type Rating = {
  level: Level | null;
  source: string | null;
  description: string;
  dependencies: string[];
  neededBy: string[];
};
export type RatingsStatus = {
  stored: boolean;
  commit: string | null;
  committedAt: string | null;
  packages: number;
};

export const levelInfo: Record<
  Level | "notRated",
  { label: string; icon: string; explanation: string }
> = {
  recommended: {
    label: "Recommended",
    icon: "check",
    explanation:
      "Pointless on most devices, or easy to get back from Google Play.",
  },
  advanced: {
    label: "Advanced",
    icon: "settings",
    explanation: "May break minor features, or has a better replacement.",
  },
  expert: {
    label: "Expert",
    icon: "warning",
    explanation:
      "Breaks important features. Removing it shouldn't stop the device starting, but that isn't guaranteed.",
  },
  unsafe: {
    label: "Unsafe",
    icon: "close",
    explanation:
      "Can break vital parts of Android and may stop the device starting.",
  },
  notRated: {
    label: "Not rated",
    icon: "info",
    explanation: "UAD has no rating for this. That means unknown, not low risk.",
  },
};

export const groupLabels: Record<Group, string> = {
  installed: "Installed by you",
  recommended: levelInfo.recommended.label,
  advanced: levelInfo.advanced.label,
  notRated: levelInfo.notRated.label,
  expert: levelInfo.expert.label,
  unsafe: levelInfo.unsafe.label,
};
