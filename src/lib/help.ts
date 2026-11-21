// Questions and answers shown on the Display page (the first four) and in Settings (all).
export type HelpItem = [question: string, answer: string];

export const HELP: HelpItem[] = [
  [
    "How do I get Luna?",
    "Download it for Windows, macOS or Linux from the project's releases page, or build it yourself from the source code with the two commands in the README. There is nothing to buy and no account to make.",
  ],
  [
    "How do I cancel or get a refund?",
    "There is nothing to cancel and nothing to pay back: Luna is free, has no subscription and never asks for payment. To remove it, quit Luna (your screens go back to normal), then uninstall it like any other app: Apps in Windows Settings, the Applications folder on macOS, or your package manager on Linux.",
  ],
  [
    "How do I contact you?",
    "Open an issue on the project's code page with what you did, what you expected, your system (Windows, macOS or Linux) and its version. Luna sends nothing about you, so a report is the only way we hear about a bug or a question.",
  ],
  [
    "Are crosshair overlays and screen filters allowed in games?",
    "Luna's crosshair is a separate see-through window drawn above the game, and its colour changes use the same system settings as night light tools, so nothing is loaded into the game. Rules still differ from one game and one tournament to another, so check the rules of the game you play before using them in ranked matches.",
  ],
  [
    "Does Luna work on macOS and Linux?",
    "Yes, with fewer tools. Gamma, brightness, contrast, red, green and blue, night vision, the crosshair, the look wheel and binds work on all three systems. On macOS, binds need the Accessibility permission (System Settings > Privacy & Security > Accessibility). On Linux, Luna needs an X11 session, and NVIDIA cards also get the driver's digital vibrance through nvidia-settings. Hue, saturation, sight colours, Ultra Zoom and the Optimizer use Windows features and only exist there.",
  ],
  [
    "Does Luna touch my games?",
    "No. Luna never opens, reads or changes a game's files or memory, and it loads nothing into a game. It only uses the screen settings your system offers to every program: the colour curve of each display, the Windows magnifier colour effects and, where the driver allows it, the NVIDIA or AMD vibrance setting.",
  ],
  [
    "Why does strong gamma look weaker than the slider?",
    "Windows limits how far a program may bend the colour curve without administrator rights. When a setting goes past that limit, Luna applies the strongest curve Windows accepts and shows a note under the sliders. macOS and Linux have no such limit.",
  ],
  [
    "How do I put my screens back to normal?",
    "Use Reset on the Display page, turn tuning off in the sidebar, or quit Luna. Every screen also goes back to normal if Luna closes unexpectedly or the computer restarts.",
  ],
  [
    "Does Luna work on AMD and Intel graphics?",
    "Yes. Luna finds the card behind each screen by itself. On Windows it uses NVIDIA's digital vibrance, AMD's saturation setting, and on Intel or any other card a colour boost that looks the same. Gamma, night vision, the crosshair and the look wheel work on every card and every system.",
  ],
  [
    "Can I undo the Optimizer?",
    "Yes. The Optimizer exists on Windows only, and Luna saves the old value of every Windows setting before it changes it. Undo all, or switching a tweak off, puts that saved value back.",
  ],
  [
    "Where are my settings, and are they sent anywhere?",
    "They are plain JSON files in Luna's settings folder (AppData on Windows, Application Support on macOS, ~/.config/luna on Linux), which you can open from the Settings page. Luna has no account and no statistics: your settings never leave your computer. The only thing it fetches online is the update check, which you can turn off in Settings.",
  ],
];

// Shown on the Zoom page.
export const ZOOM_HELP: HelpItem[] = [
  [
    "The zoom does nothing in my game. Why?",
    "Check that a key is set above, then set the game to borderless or windowed mode: in exclusive full screen, Windows may not be able to magnify the picture. If Luna says magnification is not available on this PC, the Windows magnifier itself is turned off or missing.",
  ],
  [
    "Is zooming allowed in games?",
    "Luna uses the Windows magnifier, the same one the accessibility settings use, so nothing is loaded into the game. Some games and tournaments still forbid any zoom tool, so check their rules before using it in ranked matches.",
  ],
  [
    "Why only on Windows?",
    "Ultra Zoom relies on the Windows magnifier. macOS and Linux have no equivalent that Luna can drive the same way, so the page only appears on Windows.",
  ],
];

// Shown on the Borderless page.
export const BORDERLESS_HELP: HelpItem[] = [
  [
    "The game still has a frame.",
    "Set the game to windowed mode in its own video settings first, not full screen. Some games draw their frame again when you change the resolution: Luna takes it off again by itself for remembered games, otherwise press the key again.",
  ],
  [
    "Can the game fill my ultrawide screen?",
    "By default Luna only takes the frame off and leaves the game where it was. Pick Fill the screen under Size to stretch the window across the whole screen, or 16:9 or 21:9 to centre it in the middle of the screen.",
  ],
  [
    "Nothing happens with one game.",
    "If the game runs as administrator, Windows does not let other programs change its window. Start Luna as administrator too, or start the game normally. A few anti-cheat systems also lock their window.",
  ],
  [
    "Is it safe with anti-cheat?",
    "Luna only changes the frame and position of the window through standard Windows calls, the same as moving a window by hand. Nothing is loaded into the game and its files are never touched.",
  ],
  [
    "How do I get the frame back?",
    "Press the key again, click Restore, or close the game. A window Luna changed stays that way until then, even if you quit Luna.",
  ],
];

// Shown on the Clean page.
export const CLEAN_HELP: HelpItem[] = [
  [
    "What does Luna delete?",
    "Only files that programs make again by themselves: caches, temporary files, old logs and crash reports. Your documents, downloads, game saves, passwords, browsing history and cookies are never on the list, so you stay signed in everywhere.",
  ],
  [
    "Why does some space stay after cleaning?",
    "A program that is open keeps some of its files in use, and Windows does not let anyone delete those. Close the browser, Discord or the launcher marked as open, then clean again. Temporary files from the last day also stay, in case a setup is still using them.",
  ],
  [
    "Why do some items need administrator?",
    "Windows Update downloads, the Windows temp folder and system crash dumps belong to Windows itself. Restart Luna as administrator from this page to clean them; Windows asks for permission first.",
  ],
  [
    "Should I clear the shader cache?",
    "Only when a game shows graphics glitches, or after a driver update. Games build their shaders again on the next launch, which can stutter for a few minutes, so it is not ticked by default.",
  ],
  [
    "Why does the space fill up again?",
    "Caches grow back as you use your programs, and that is normal: it is what keeps pages and games loading fast. Cleaning once a month, or when the disk is nearly full, is plenty.",
  ],
];

export const GAMES_HELP: HelpItem[] = [
  [
    "When does a profile start?",
    "Luna looks at the running programs every two seconds. A couple of seconds after the game opens, its gamma, colour, crosshair and screen mode are set. When it closes, or if Luna quits or crashes, everything goes back.",
  ],
  [
    "The resolution did not change.",
    "Only the modes the screen offers are listed. A game in exclusive full screen sets its own resolution and wins: use windowed or borderless mode, or pick the same mode in the game.",
  ],
  [
    "The FPS counter shows dashes.",
    "It counts the game in front, and only Direct3D 9 to 12 games. Vulkan and OpenGL games are not counted yet. Windows also asks for Luna to run as administrator, and an overlay cannot show over exclusive full screen.",
  ],
  [
    "Is Potato Mode safe?",
    "It only changes priorities and five Optimizer switches that undo cleanly, and it writes down what it changed. Turning it off, quitting Luna or the next start after a crash puts everything back.",
  ],
];
