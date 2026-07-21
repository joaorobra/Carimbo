// English UI strings — the default language and the fallback for any key a
// translation is missing. Keys are flat and dot-namespaced by area. Placeholders
// use {name} and are filled by t(key, params).

export const en = {
  // --- app shell / nav ---
  "app.name": "Carimbo",
  "nav.sections": "Sections",
  "nav.snippets": "Snippets",
  "nav.clipboard": "Clipboard",
  "nav.colors": "Colors",
  "nav.settings": "Settings",
  "nav.folders": "Folders",

  // --- window title bar (custom chrome) ---
  "titlebar.minimize": "Minimize",
  "titlebar.maximize": "Maximize",
  "titlebar.restore": "Restore",
  "titlebar.close": "Close",

  // --- snippet list ---
  "snippets.searchPlaceholder": "Search snippets…",
  "snippets.searchAria": "Search snippets",
  "snippets.newTitle": "New snippet (Ctrl+N)",
  "snippets.emptyNoResults": 'No snippets found for "{query}".',
  "snippets.emptyNone": "No snippets yet.",
  "snippets.createFirst": "Create the first one",
  "snippets.placeholderSelect": "Select a snippet or create a new one.",
  "snippets.favAdd": "Add to favorites",
  "snippets.favRemove": "Remove from favorites",

  // --- snippet editor ---
  "editor.name": "Name",
  "editor.namePlaceholder": "e.g. My email",
  "editor.trigger": "Shortcut",
  "editor.optional": "(optional)",
  "editor.triggerPlaceholder": "e.g. ;email",
  "editor.folder": "Folder",
  "editor.noFolder": "— No folder —",
  "editor.content": "Content",
  "editor.insertTokenAria": "Insert dynamic token",
  "editor.bodyPlaceholder": "Text that will be inserted…",
  "editor.preview": "Preview",
  "editor.save": "Save",
  "editor.cancel": "Cancel",
  "editor.delete": "Delete",
  "editor.confirmDelete": 'Delete the snippet "{name}"?',
  "editor.errorSave": "Could not save",
  "editor.errorDuplicateTrigger":
    'The shortcut "{trigger}" is already used by another snippet.',
  "editor.richText": "Rich text",
  "editor.richToolbar": "Formatting",
  "editor.richNote":
    "Formatting is inserted in apps that support it (email, docs); others receive plain text. Dynamic tokens still work.",
  "editor.bold": "Bold",
  "editor.italic": "Italic",
  "editor.underline": "Underline",
  "editor.link": "Add link",
  "editor.linkPrompt": "Link URL:",
  "editor.clearFormat": "Clear formatting",

  // --- tokens (dynamic insert buttons + descriptions) ---
  "token.date.label": "Date",
  "token.date.desc": "Today's date",
  "token.time.label": "Time",
  "token.time.desc": "Current time (hh:mm)",
  "token.datetime.label": "Date & time",
  "token.datetime.desc": "Date and time",
  "token.clipboard.label": "Clipboard",
  "token.clipboard.desc": "Current clipboard contents",
  "token.cursor.label": "Cursor",
  "token.cursor.desc":
    "Where the cursor lands after inserting. e.g. Dear {cursor},",
  "token.uuid.label": "UUID",
  "token.uuid.desc": "A fresh random unique identifier",
  "token.field.label": "Form field",
  "token.field.desc":
    "A field the user fills in on insert. e.g. [[client_name:Client name]]",

  // --- folders ---
  "folders.all": "All",
  "folders.newPlaceholder": "Folder name",
  "folders.new": "New folder",
  "folders.errorCreate": "Could not create folder",
  "folders.confirmDelete":
    'Delete the folder "{name}"? Its snippets will not be deleted.',
  "folders.deleteTitle": "Delete folder",
  "folders.deleteAria": "Delete folder {name}",

  // --- clipboard history ---
  "clipboard.searchPlaceholder": "Search history…",
  "clipboard.searchAria": "Search clipboard history",
  "clipboard.emptyNoResults": "Nothing found in history.",
  "clipboard.emptyNone":
    "Your clipboard history will appear here as you copy.",
  "clipboard.pin": "Pin",
  "clipboard.unpin": "Unpin",
  "clipboard.copy": "Copy",
  "clipboard.copyAria": "Copy to clipboard",
  "clipboard.delete": "Delete",
  "clipboard.deleteAria": "Delete from history",
  "clipboard.copied": "Copied",

  // clip content-type badges
  "clipboard.type.url": "Link",
  "clipboard.type.email": "Email",
  "clipboard.type.color": "Color",
  "clipboard.type.path": "File path",
  "clipboard.type.files": "Files",

  // clip actions
  "clipboard.openUrl": "Open link",
  "clipboard.openUrlAria": "Open link in browser",
  "clipboard.email": "Compose email",
  "clipboard.reveal": "Show in folder",
  "clipboard.revealAria": "Show file in Explorer",
  "clipboard.promote": "Save as snippet",
  "clipboard.promoteAria": "Save this clip as a reusable snippet",
  "clipboard.promoted": 'Saved snippet "{name}".',
  "clipboard.moreActions": "More actions",
  "clipboard.sourceApp": "Copied from {app}",
  // dynamic, content-aware actions in the palette row's more-options menu
  "clipboard.actions": "Actions",
  "clipboard.pasteHex": "Paste as HEX",
  "clipboard.pasteRgb": "Paste as RGB",
  "clipboard.pasteHsl": "Paste as HSL",
  "clipboard.editImage": "Edit image",
  "clipboard.soon": "Soon",

  // image lightbox / preview
  "lightbox.open": "Preview image",
  "lightbox.close": "Close",
  "lightbox.fit": "Fit to window",
  "lightbox.actualSize": "Actual size",
  "lightbox.paste": "Paste image",
  "lightbox.save": "Show file in Explorer",

  // "paste as… / copy as…" transform menu
  "clipboard.transform": "Transform",
  "clipboard.transformAria": "Copy with a transform",
  "clipboard.pasteAs": "Paste as…",
  "clipboard.copyAs": "Copy as…",
  "transform.plain": "Plain text",
  "transform.upperCase": "UPPERCASE",
  "transform.lowerCase": "lowercase",
  "transform.titleCase": "Title Case",
  "transform.trim": "Trim spaces",
  "transform.singleLine": "Single line",
  "transform.slug": "slug-form",
  "transform.base64Encode": "Base64 encode",
  "transform.base64Decode": "Base64 decode",

  // clipboard folders
  "clipboard.allFolders": "All",
  "clipboard.moveToFolder": "Move to folder",
  "clipboard.removeFromFolder": "— No folder —",
  "clipboard.newFolder": "New folder",
  "clipboard.folderPlaceholder": "Folder name",

  // --- relative time ---
  "time.now": "now",
  "time.minutes": "{n} min",
  "time.hours": "{n} h",
  "time.days": "{n} d",

  // --- settings ---
  "settings.appearance": "Appearance",
  "settings.theme": "Theme",
  "settings.theme.system": "Automatic (OS)",
  "settings.theme.light": "Light",
  "settings.theme.dark": "Dark",
  "settings.theme.hcLight": "High contrast (light)",
  "settings.theme.hcDark": "High contrast (dark)",
  "settings.fontSize": "Font size: {pct}%",
  "settings.glassOpacity": "Palette transparency: {pct}%",
  "settings.glassOpacityNote":
    "How see-through the Quick Search window is. Lower it for more of the frosted-glass effect, or raise it to 100% for a fully solid background.",
  "settings.density": "Density",
  "settings.density.compact": "Compact",
  "settings.density.comfortable": "Comfortable",
  "settings.reduceMotion": "Reduce animations",

  "settings.language": "Language & region",
  "settings.uiLanguage": "Interface language",
  "settings.language.en": "English",
  "settings.language.ptBR": "Portuguese (Brazil)",
  "settings.region": "Region",
  "settings.region.us": "United States",
  "settings.region.br": "Brazil",
  "settings.region.note":
    "Sets the date format for {date}: United States is mm/dd/yyyy, Brazil is dd/mm/yyyy.",

  "settings.quickSearch": "Quick search",
  "settings.globalHotkey": "Global shortcut",
  "settings.hotkeyNote":
    "Opens quick search in any app. Click the shortcut and press a new combination (e.g. {combo}). If the combination is already used by another program, choose another.",
  "settings.mainTab": "Main tab",
  "settings.mainTabNote":
    "Which tab the shortcut opens first. The second shortcut opens the other one.",
  "settings.opensTab": "opens {tab}",
  "settings.secondHotkey": "Second shortcut (optional)",
  "settings.secondHotkeyNote":
    "Set a second combination to jump straight to {tab}. Leave empty to use just one shortcut.",

  "settings.colorHotkey": "Color picker shortcut",
  "settings.colorHotkeyNote":
    "Press this combination in any app to grab a color from the screen. After you click, Carimbo opens on the Colors page with the color loaded. Leave empty to disable.",

  "settings.expansion": "Shortcut expansion",
  "settings.expansionEnable": "Enable automatic shortcut expansion",
  "settings.expansionNote":
    "Type a shortcut (e.g. {trigger}) in any app and it is replaced by the snippet's text. Requires installing a keyboard monitor — some antivirus tools may warn. Stays off until you turn it on here.",
  "settings.injectMethod": "Insertion method",
  "settings.injectPaste": "Paste (fast)",
  "settings.injectType": "Type (compatible)",
  "settings.injectNote":
    '"Paste" is faster and preserves your clipboard. "Type" works in apps that block pasting (some terminals).',

  "settings.excludedApps": "Don't expand in these apps",
  "settings.excludedAppsNote":
    "Shortcuts won't expand while these apps are focused — useful for password managers, terminals, or games. Enter the program's executable name (e.g. KeePass.exe).",
  "settings.excludedAdd": "Add",
  "settings.excludedPlaceholder": "e.g. KeePass.exe",
  "settings.excludedRemove": "Remove {app}",
  "settings.excludedSuggestions": "From your clipboard history:",

  "settings.clipboard": "Clipboard",
  "settings.retentionDays": "Keep history for: {n} day{plural}",
  "settings.retentionMax": "Maximum items: {n}",
  "settings.retentionNote":
    "Pinned items and items in folders are never removed automatically. Password managers that mark content as sensitive are ignored.",

  "settings.backup": "Backup & restore",
  "settings.backupNote":
    "Save all your snippets and folders to a file, or restore them from one. Importing adds to your library — nothing is overwritten or deleted.",
  "settings.backupExport": "Export to file…",
  "settings.backupImport": "Import from file…",
  "settings.backupExported": "Backup saved.",
  "settings.backupImported":
    "Imported {snippets} snippet(s) and {folders} folder(s). {dropped} shortcut(s) skipped (already in use).",
  "settings.backupError": "Something went wrong. Please try again.",

  "settings.import": "Import from another app",
  "settings.importNote":
    "Moving from another expander? Import your snippets from espanso (.yml), a CSV of shortcut,text pairs (TextExpander, aText, Beeftext), or a JSON list. Adds to your library — nothing is overwritten.",
  "settings.importButton": "Import from file…",
  "settings.importDone":
    "Imported {snippets} snippet(s). {dropped} shortcut(s) skipped (already in use); {skipped} row(s) had no text.",
  "settings.importEmpty":
    "No snippets found in that file. Check it's an espanso, CSV, or JSON export.",

  "settings.cloud": "Cloud & account",
  "settings.cloudDesc": "Cross-device sync and cloud backup.",
  "settings.comingSoon": "Coming soon",

  // --- hotkey recorder ---
  "hotkey.recordAria": "Record quick-search shortcut",
  "hotkey.press": "Press the combination…",
  "hotkey.restoreDefault": "Restore default",
  "hotkey.errorSet": "Could not set this shortcut.",
  "hotkey.none": "Not set",
  "hotkey.clear": "Remove shortcut",

  // --- palette (quick popup) ---
  "palette.searchSnippet": "Search snippet",
  "palette.searchSnippetPlaceholder": "Search snippet…",
  "palette.searchClipboard": "Search clipboard",
  "palette.searchClipboardPlaceholder": "Search history…",
  "palette.emptyNoSnippets": "No snippets found",
  "palette.emptyNoSnippetsYet": "No snippets yet",
  "palette.emptyNoClips": "Nothing found in history",
  "palette.emptyNoClipsYet": "Your history will appear here",
  "palette.emptyNoSnippetsHint": "Create snippets in Carimbo, then insert them here.",
  "palette.emptyNoClipsHint": "Copy something and it'll show up here.",
  "palette.emptyNoResultsHint": "Try a different search.",
  "palette.insertError": "Couldn't insert — try again.",
  "palette.navigate": "navigate",
  "palette.switchTab": "switch tab",
  "palette.insert": "insert",
  "palette.paste": "paste",
  "palette.close": "close",
  "palette.results": "{n} result{plural}",
  "palette.pinAdd": "Pin to favorites",
  "palette.pinRemove": "Unpin from favorites",
  "palette.moreOptions": "More options",
  "palette.insertAs": "Insert as…",
  "palette.pasteAs": "Paste as…",
  // Section headers shown (no active query) when favorites are floated to the top.
  "palette.favorites": "Favorites",
  "palette.otherSnippets": "Others",

  // --- variable form ---
  "form.fillFields": "Fill in fields",
  "form.back": "Back (Esc)",
  "form.backAria": "Back to list",
  "form.fieldPlaceholder": "Enter {label}…",
  "form.nextField": "next field",
  "form.insert": "insert",
  "form.backHint": "back",
  "form.insertBtn": "Insert",

  // --- radial disambiguation ---
  "radial.title": "Several similar triggers",
  "radial.chooseAria": "Choose snippet",
  "radial.navigate": "navigate",
  "radial.choose": "choose",
  "radial.insert": "insert",
  "radial.close": "close",

  // --- color picker ---
  "colors.title": "Colors",
  "colors.pick": "Pick from screen",
  "colors.pickAria": "Pick a color from the screen",
  "colors.picking": "Click anywhere to pick a color…",
  "colors.overlayHint": "Click to pick · Esc cancels",
  "colors.edit": "Adjust",
  "colors.hex": "Hex",
  "colors.red": "Red",
  "colors.green": "Green",
  "colors.blue": "Blue",
  "colors.hue": "Hue",
  "colors.saturation": "Saturation",
  "colors.lightness": "Lightness",
  "colors.scrollAdjust": "Scroll to adjust",
  "colors.scrollAdjustNote":
    "Hover a field or slider and scroll the mouse wheel to change it by 1 (hold Shift for 10).",
  "colors.tones": "Tones",
  "colors.darker": "Darker",
  "colors.lighter": "Lighter",
  "colors.baseTone": "Current color",
  "colors.toneAria": "Select tone {value}",
  "colors.copyFormats": "Copy as",
  "colors.copyAria": "Copy {format} value",
  "colors.copied": "{value} copied.",
  "colors.copyError": "Couldn't copy — try again.",
  "colors.recent": "Recent",
  "colors.recentAria": "Use recent color {value}",

  // --- toasts ---
  "toast.elevatedBlocked":
    "Can't expand in an app running as administrator.",

  // --- onboarding ---
  "onboarding.welcomeAria": "Welcome to Carimbo",
  "onboarding.skipAria": "Skip introduction",
  "onboarding.title": "Welcome to Carimbo",
  "onboarding.lead":
    "Save text you use all the time — names, IDs, addresses, signatures — and insert it anywhere in an instant. Let's create your first one.",
  "onboarding.name": "Name",
  "onboarding.namePlaceholder": "e.g. My email",
  "onboarding.text": "Text",
  "onboarding.textPlaceholder": "e.g. you@example.com",
  "onboarding.trigger": "Shortcut",
  "onboarding.triggerPlaceholder": "e.g. ;email",
  "onboarding.tip":
    "Press {combo} anywhere to search and insert.",
  "onboarding.skip": "Skip",
  "onboarding.create": "Create my first snippet",
  "onboarding.error": "Could not create the snippet. Try another shortcut.",

  // --- first-run region picker ---
  "region.title": "Welcome to Carimbo",
  "region.lead": "Where will you be using Carimbo? This sets your date format and the example snippets we start you with. You can change it later in Settings.",
  "region.us": "United States",
  "region.usDesc": "English interface · mm/dd/yyyy dates",
  "region.br": "Brazil",
  "region.brDesc": "Portuguese interface · dd/mm/yyyy dates",
  "region.continue": "Continue",
} as const;

export type StringKey = keyof typeof en;
