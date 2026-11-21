// Questions and answers shown on the Display page (the first four) and in Settings (all).
export type HelpItem = [question: string, answer: string];

export const HELP: HelpItem[] = [
  [
    "How do I get Luna?",
    "Download the installer from the project's releases page, or build it yourself from the source code with the two commands in the README. There is nothing to buy and no account to make.",
  ],
  [
    "How do I cancel or get a refund?",
    "There is nothing to cancel and nothing to pay back: Luna is free, has no subscription and never asks for payment. To remove it, quit Luna from the tray (your screens go back to normal), then uninstall it from Apps in Windows Settings.",
  ],
  [
    "How do I contact you?",
    "Open an issue on the project's code page with what you did, what you expected and your Windows version. Luna never sends anything by itself, so a report is the only way we hear about a bug or a question.",
  ],
  [
    "Are crosshair overlays and screen filters allowed in games?",
    "Luna's crosshair is a separate see-through window drawn above the game, and its colour changes use the same Windows settings as night light tools, so nothing is loaded into the game. Rules still differ from one game and one tournament to another, so check the rules of the game you play before using them in ranked matches.",
  ],
  [
    "Does Luna touch my games?",
    "No. Luna never opens, reads or changes a game's files or memory, and it loads nothing into a game. It only uses the screen settings Windows offers to every program: the colour curve of each display, the Windows magnifier colour effects and, on NVIDIA and AMD cards, the driver's own vibrance or saturation setting.",
  ],
  [
    "Why does strong gamma look weaker than the slider?",
    "Windows limits how far a program may bend the colour curve without administrator rights. When a setting goes past that limit, Luna applies the strongest curve Windows accepts and shows a note under the sliders.",
  ],
  [
    "How do I put my screens back to normal?",
    "Use Reset on the Display page, turn tuning off in the sidebar, or quit Luna. Every screen also goes back to normal if Luna closes unexpectedly or the PC restarts.",
  ],
  [
    "Does Luna work on AMD and Intel graphics?",
    "Yes. Luna finds the card behind each screen by itself. On NVIDIA it uses the driver's digital vibrance, on AMD the driver's saturation setting, and on Intel or any other card a colour boost that looks the same. Gamma, zoom, night vision, the crosshair and the look wheel work on every card.",
  ],
  [
    "Can I undo the Optimizer?",
    "Yes. Luna saves the old value of every Windows setting before it changes it. Undo all, or switching a tweak off, puts that saved value back.",
  ],
  [
    "Where are my settings, and are they sent anywhere?",
    "They are plain JSON files in the Luna folder of your AppData, which you can open from the Settings page. Luna has no account, no server and no statistics: nothing leaves your PC.",
  ],
];
