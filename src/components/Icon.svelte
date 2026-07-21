<script lang="ts">
  // Icon set backed by Lucide (@lucide/svelte). Stroke inherits currentColor.
  // Kept behind this component so call sites stay on the same `name`/`size` API.
  import Search from "@lucide/svelte/icons/search";
  import Plus from "@lucide/svelte/icons/plus";
  import Star from "@lucide/svelte/icons/star";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Folder from "@lucide/svelte/icons/folder";
  import Settings from "@lucide/svelte/icons/settings";
  import X from "@lucide/svelte/icons/x";
  import Clipboard from "@lucide/svelte/icons/clipboard";
  import FileText from "@lucide/svelte/icons/file-text";
  import Check from "@lucide/svelte/icons/check";
  import Stamp from "@lucide/svelte/icons/stamp";
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import Mail from "@lucide/svelte/icons/mail";
  import Wand from "@lucide/svelte/icons/wand-sparkles";
  import MoreHorizontal from "@lucide/svelte/icons/ellipsis";
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import FolderInput from "@lucide/svelte/icons/folder-input";
  import Copy from "@lucide/svelte/icons/copy";
  import Download from "@lucide/svelte/icons/download";
  import Maximize from "@lucide/svelte/icons/maximize-2";
  import Pipette from "@lucide/svelte/icons/pipette";
  import Pin from "@lucide/svelte/icons/pin";
  import Minus from "@lucide/svelte/icons/minus";
  import Square from "@lucide/svelte/icons/square";
  import Minimize2 from "@lucide/svelte/icons/minimize-2";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Sparkles from "@lucide/svelte/icons/sparkles";
  import Hash from "@lucide/svelte/icons/hash";
  import Pencil from "@lucide/svelte/icons/pencil";
  import type { Component } from "svelte";

  interface Props {
    name:
      | "search"
      | "plus"
      | "star"
      | "star-filled"
      | "trash"
      | "folder"
      | "settings"
      | "close"
      | "clipboard"
      | "snippet"
      | "check"
      | "stamp"
      | "back"
      | "external-link"
      | "mail"
      | "wand"
      | "more"
      | "file-plus"
      | "folder-input"
      | "copy"
      | "download"
      | "maximize" // expand-to-view-full-size (lightbox/clip preview), NOT window chrome
      | "pipette"
      | "pin" // pinned clip — distinct glyph from "star" (favorite)
      | "minimize" // window chrome: minimize-to-taskbar
      | "window-maximize" // window chrome: maximize
      | "restore" // window chrome: restore-down from maximized
      | "window-close" // window chrome: close button (reuses the "close" glyph)
      | "chevron-right"
      | "sparkles"
      | "hash" // color-format action (paste as HEX/RGB/HSL)
      | "edit"; // edit an image clip (deferred)
    size?: number;
  }
  let { name, size = 18 }: Props = $props();

  const icons = {
    search: Search,
    plus: Plus,
    star: Star,
    "star-filled": Star,
    trash: Trash2,
    folder: Folder,
    settings: Settings,
    close: X,
    clipboard: Clipboard,
    snippet: FileText,
    check: Check,
    stamp: Stamp,
    back: ArrowLeft,
    "external-link": ExternalLink,
    mail: Mail,
    wand: Wand,
    more: MoreHorizontal,
    "file-plus": FilePlus,
    "folder-input": FolderInput,
    copy: Copy,
    download: Download,
    maximize: Maximize,
    pipette: Pipette,
    pin: Pin,
    minimize: Minus,
    "window-maximize": Square,
    restore: Minimize2,
    "window-close": X,
    "chevron-right": ChevronRight,
    sparkles: Sparkles,
    hash: Hash,
    edit: Pencil,
  } satisfies Record<Props["name"], Component>;

  const Glyph = $derived(icons[name]);
  // Lucide has no separate filled star; fill the outline for the "active" state.
  const fill = $derived(name === "star-filled" ? "currentColor" : "none");
</script>

<Glyph {size} {fill} aria-hidden="true" focusable="false" />
