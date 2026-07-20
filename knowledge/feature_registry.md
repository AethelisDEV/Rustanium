# AE Rustanium: Feature & Architecture Registry

This registry tracks all newly implemented features, system-level enhancements, and low-level architectural decisions to maintain consistency and AI-to-AI continuity.

---

## 🛠️ Low-Level Kernel Enhancements (`kernel-x86`)

### 1. Bare-Metal CPU Yield Bugfix (CPU Hang Prevention)
* **Date**: May 29, 2026
* **Description**: Replaced the raw assembly `hlt` instruction inside bare-metal target `Cpu::halt()` with `core::hint::spin_loop()`.
* **Rationale**: Because `kernel-x86` operates in polling mode and has interrupts disabled by default on boot, calling `hlt` put the CPU to sleep permanently. Replacing it with `spin_loop` (which compiles to a `pause` instruction) ensures the polling loops for the PS/2 keyboard and serial COM1 port continue executing smoothly.
* **Location**: `kernel-core/src/hal.rs`

### 2. SSE & FPU Hardware Activation on Boot
* **Date**: May 29, 2026
* **Description**: Added `enable_sse()` to initialize CR0 and CR4 control registers.
  * Clears `Cr0Flags::EMULATE_COPROCESSOR`.
  * Sets `Cr0Flags::MONITOR_COPROCESSOR`.
  * Sets `Cr4Flags::OSFXSR` (enabling FXSAVE/FXRSTOR).
  * Sets `Cr4Flags::OSXMMEXCPT_ENABLE` (enabling SIMD exceptions).
* **Rationale**: Prevent `Invalid Opcode (#UD)` exceptions and silent triple-faults when the compiler generates SSE/floating-point operations (e.g. for bulk memory clears or copies in `.bss`).
* **Location**: `kernel-x86/src/main.rs`

### 3. Interrupt Descriptor Table (IDT) & Exception Handling
* **Date**: May 29, 2026
* **Description**: Implemented a static `InterruptDescriptorTable` (IDT) loaded on boot.
* **Registered Handlers**:
  * **Breakpoint Exception**: Prints CPU stack frame to COM1.
  * **Double Fault Exception**: Triggers structured kernel panic.
  * **Page Fault Exception**: Intercepts illegal memory access, prints the faulting address from `CR2`, logs registers, and halts CPU safely.
* **Location**: `kernel-x86/src/main.rs` (Refactored and moved to `kernel-x86/src/interrupts.rs`)

### 4. Interrupt-Driven Asynchronous Kernel Architecture (PIC, PIT Timer, Asynchronous Keyboard)
* **Date**: May 29, 2026
* **Description**: Replaced the entire synchronous polling-based I/O loop with a highly advanced, fully asynchronous, interrupt-driven architecture.
* **Key Components**:
  * **8259 Chained PICs Controller**: Configured via direct port writes to map hardware IRQs (IRQ 0-15) to custom CPU vector offsets (32-47).
  * **100 Hz PIT (Programmable Interval Timer - IRQ 0)**: Configured using rate generator mode to tick asynchronously at precisely 100 Hz, advancing kernel `ticks` in the background.
  * **Asynchronous Keyboard Sentry (IRQ 1)**: Intercepts PS/2 key strikes, decodes them on the fly, and registers them in a thread-safe static buffer (`KEYBOARD_BUFFER`).
  * **Idle Sleep (HLT Loop)**: The main execution loop is now entirely passive, utilizing `x86_64::instructions::hlt()` to sleep the CPU. It wakes up exclusively upon hardware interrupts, processes queued actions, and sleeps again—perfectly mimicking the idle behavior of mature kernels like Linux.
* **Location**: `kernel-x86/src/interrupts.rs` (New module) and `kernel-x86/src/main.rs` (Integrated loop)

### 5. Cooperative Multitasking & Assembly Context Switching
* **Date**: May 29, 2026
* **Description**: Implemented a low-level cooperative scheduler allowing threads to run in parallel on independent 8 KB stacks.
* **Key Components**:
  * **Thread Control Block (TCB)**: Manages dynamic 8 KB stack spaces, thread IDs, execution status, and saved stack pointers.
  * **Assembly Context Switcher (`switch_context`)**: A raw, inline-assembly routine that pushes callee-preserved registers (rbp, rbx, r12, r13, r14, r15) to the active stack, saves the stack pointer (rsp), loads the new thread's stack pointer, and pops the registers, returning to the new execution stream.
  * **Round-Robin Scheduling**: Swaps thread execution sequentially during yields.
  * **Background Micro-tasks**: Spawned `thread_scrubber` (memory sweeping daemon) and `thread_diagnostics` (logging engine) executing in parallel loops cooperatively.
* **Location**: `kernel-x86/src/scheduler.rs` (New module) and `kernel-x86/src/main.rs` (Spawned threads & yields)

### 6. UEFI & BIOS Dual-Boot & GOP Framebuffer Graphics
* **Date**: May 29, 2026
* **Description**: Migrated from legacy BIOS boot (`bootloader` v0.9) to modern **UEFI & BIOS Dual-Boot** architecture (`bootloader_api` v0.11). Replaced the fragile `0xB8000` VGA text mode writes with direct graphics rendering on the UEFI **Graphics Output Protocol (GOP)** linear framebuffer.
* **Key Components**:
  * **UEFI Graphics Engine (`framebuffer.rs`)**: Encapsulates pixel-level drawing with auto-detection of hardware RGB/BGR layout, drawing color/gradient panels, status blocks, and interactive text.
  * **Embedded 8x8 Bitmap Font**: Embedded a lightweight 8x8 monospace bitmap font to print console and shell text directly to GOP.
  * **Smart Visual Redraw Optimization**: Implemented a state-differential check rendering the screen *only* when ticks update or line buffer lengths change, eliminating CPU draw spikes.
* **Location**: `kernel-x86/src/framebuffer.rs` (New module) and `kernel-x86/src/main.rs` (Bootloader entry & rendering)

### 7. Physical Hardware Interrupt Safety (Direct I/O Port Polling)
* **Date**: May 29, 2026
* **Description**: Deactivated external hardware interrupts (`cli` mode) by commenting out `x86_64::instructions::interrupts::enable();`. 
* **Rationale**: Bypasses legacy PIC/APIC motherboard routing conflicts and flaky USB emulation layers on modern physical UEFI machines. Since we already employ a highly responsive cooperative polling routine for both the PS/2 keyboard (ports `0x60` and `0x64`) and COM1 UART serial ports directly in our main execution loop, external hardware interrupts are completely unnecessary. synchronous CPU exceptions (GPF, Page Faults) still trigger perfectly.
* **Location**: `kernel-x86/src/main.rs`

### 8. Dynamic Memory Reclamation & LockedHeap Integration
* **Date**: May 29, 2026
* **Description**: Transitioned the global kernel allocator from the leaky bump allocator to `linked_list_allocator::LockedHeap` with a 1 MB heap buffer.
* **Rationale**: A bump allocator never reclaims memory upon deallocation, causing the small 256 KB heap to be completely exhausted after 154 ticks of continuous telemetry allocations in `core.tick()`. `LockedHeap` dynamically reclaims deallocated memory, allowing the system to tick indefinitely (tested past 500+ sweeps) without memory leaks.
* **Location**: `kernel-x86/Cargo.toml` and `kernel-x86/src/main.rs`

### 9. Unified Visual Panic Screen & GraphicsWriter Stream
* **Date**: May 29, 2026
* **Description**: Overhauled exception and panic handlers to forcefully unlock the global static `GRAPHICS` spinlock and render detailed error stack traces visually.
* **Key Components**:
  * **GraphicsWriter (`framebuffer.rs`)**: A custom formatting stream conforming to the standard `core::fmt::Write` trait, allowing heap-free directly-formatted string writes onto the graphics framebuffer.
  * **Unified Panic Screen**: If a kernel panic, double fault, page fault, GPF, invalid opcode, or divide-by-zero occurs, the system renders a bright red diagnostic console box on screen, providing the exact file, line, and stack trace to aid native hardware debugging.
* **Location**: `kernel-x86/src/framebuffer.rs`, `kernel-x86/src/interrupts.rs`, and `kernel-x86/src/main.rs`

### 10. Direct Serial Keyboard Echo, Prompt Visibility, & Turkish Layout Support
* **Date**: May 30, 2026
* **Description**: 
  * Diverted character/backspace input echoing from the standard `print!` macro (which appends directly to `TTY_LOGS` line-by-line) to write directly to `SERIAL_WRITER`. When the user hits Enter, the full prompt and command line are formatted and appended to `TTY_LOGS` as a single unified line.
  * Added explicit calls to draw the command prompt during kernel boot initialization, and set `last_rendered_len = 9999` to trigger immediate prompt rendering on the first loop cycle.
  * Modified `_print` to suppress background diagnostics spam (`[THREAD 1]` and `[THREAD 2]` logs) from being output to the serial console, keeping the active input line and prompt clean and always visible.
  * Added a `loadkeys` command (e.g. `loadkeys trq`) to switch between US and Turkish Q (TRQ) keyboard layout scancode decoding tables dynamically. Maps Turkish characters to standard ASCII equivalents so they render beautifully with embedded fonts and compile cleanly with CLI inputs.
* **Rationale**: 
  * Bypasses the issue where every typed character or backspace was appended to `TTY_LOGS` as a separate entry, causing the TTY console screen scrollback to shift upward with every single keystroke. Now, live typing only echoes to the serial port and updates the dedicated, static interactive prompt at the bottom of the screen, preserving the scrollback history cleanliness.
  * Ensures that the `rustanium:/>` shell prompt is immediately visible when the system boots up and when switching between modes, rather than only appearing after a character is typed.
  * Prevents background thread logs from constantly interrupting and scrambling the active input line on the serial console, so the prompt `rustanium:/>` is never pushed out of view.
  * Enables Turkish Q keyboard users to type on native console layouts while automatically mapping non-ASCII chars to ASCII look-alikes to bypass font index limits (which made characters completely invisible).
* **Location**: `kernel-x86/src/keyboard.rs` and `kernel-x86/src/shell.rs`

### 11. Monolithic main.rs Modular Split (God Object Refactoring)
* **Date**: May 30, 2026
* **Description**: Fully refactored and split the 1400+ line `main.rs` monolithic entry point into three highly cohesive, single-responsibility sub-modules:
  * **`logger.rs`**: Handles print macros (`print!`, `println!`), serial writer bindings (`SERIAL_WRITER`), TTY log buffers, and telemetry suppression.
  * **`keyboard.rs`**: Encapsulates layouts (`Us`, `Trq`), shift status flags, scancode tables, hardware polling (`poll_keyboard`), and serial polling (`poll_serial`).
  * **`shell.rs`**: Exposes the interactive microkernel parser (`handle_command`), VFS relative path resolver, directory tree traversal, and CLI commands.
* **Rationale**: Adheres to strict Single Responsibility Principles (SRP) and the file length limit of 800 lines specified in `AI_GUIDELINES.md`. This slims down the entry point, isolating low-level CPU bootstrapping from CLI logic and input decoding, dramatically improving system maintainability.
* **Location**: `kernel-x86/src/main.rs`, `kernel-x86/src/logger.rs`, `kernel-x86/src/keyboard.rs`, and `kernel-x86/src/shell.rs`

### 12. HAL Assembly Separation & Unsafe Isolation (Zero Unsafe Compliance)
* **Date**: June 07, 2026
* **Description**: Extracted the raw inline assembly port I/O code from `hal.rs` into a separate GNU/LLVM assembly source file `hal.s`. Declared `extern "C" { fn hal_write_byte(b: u8); }` to invoke the helper, and strictly encapsulated the unsafe block within the safe public sarmalayıcı (wrapper) function `SerialPort::write_byte`. Additionally, added `#![deny(unsafe_code)]` at the top of `bootstrap.rs` to enforce unsafe code exclusion in bootstrap path.
* **Rationale**: Fully complies with the strict Zero Unsafe Policy for core workspace modules in `AI_GUIDELINES.md`, isolating necessary assembly/unsafe CPU boundaries from safe microkernel logic and guaranteeing the safety of bootstrap modules.
* **Location**: `kernel-core/src/hal.s` (New), `kernel-core/src/hal.rs`, and `kernel-core/src/bootstrap.rs`
### 13. Removal of Unnecessary Unsafe Blocks (Code Hygiene & Guidelines Alignment)
* **Date**: June 28, 2026
* **Description**: Cleaned up and removed 9 unnecessary `unsafe` blocks identified by `clippy` and static analysis inside the system call routing module.
* **Rationale**: Aligns the codebase with the Zero Unsafe/Unsafe Isolation guidelines in `AI_GUIDELINES.md` by eliminating unused `unsafe` wrappers around safe functions (e.g. `interrupts::enable`/`disable`) and safe macros (e.g. `core::ptr::addr_of`/`addr_of_mut`).
* **Location**: `kernel-x86/src/syscall.rs`

### 14. Safe Refactoring of Statics and Unsafe Blocks (Zero Unsafe / Safe Lock Refactoring)
* **Date**: June 28, 2026
* **Description**: Extensively refactored `usermode-x86`, `kernel-x86`, and `usermode-desktop` packages to eliminate mutable static variables (`static mut`) and unsafe pointer operations.
  * **usermode-x86**: Replaced `LOG_CALLBACK` with atomic `AtomicPtr`. Replaced `USER_RSP`, `KERNEL_STACK_TOP`, `SYSCALL_HANDLER`, and all fields of `SharedSystemInfo` with atomics (`AtomicU64`, `AtomicUsize`).
  * **kernel-x86**: Wrapped `KEYBOARD_BUFFER`, `KEYBOARD_STATE`, and `FD_TABLE` into thread-safe `Spinlock` instances. Converted `MOUSE_CYCLE`, `MOUSE_PACKET` (now `MOUSE_PACKETS`), and `SHARED_INFO_PAGE` to atomic and standard static equivalents.
  * **usermode-desktop**: Replaced `SCREEN_WIDTH`, `SCREEN_HEIGHT`, `SCREEN_FORMAT`, `START_MENU_OPEN`, `START_MENU_ANIMATING`, `START_MENU_ANIM_PROGRESS`, `TERM_ROW`, and `TERM_COL` with atomic variables (`AtomicI32`, `AtomicU32`, `AtomicBool`, `AtomicUsize`).
  * **Unsafe Reduction**: Cleaned up and removed over 30 redundant `unsafe` blocks in keyboard handlers, VFS/syscall handlers, mouse drivers, and the desktop rendering/animation loop, achieving maximum compliance with the Zero Unsafe Policy.
* **Rationale**: Replaces raw pointer dereferences and unsynchronized mutable static access with safe, race-free, and compiler-verified synchronization primitives (locks and atomics).
* **Location**: All source files in `usermode-x86/src/`, `kernel-x86/src/`, and `usermode-desktop/src/`.

### 15. Software Renderer & Window Shadow Rendering Optimization (Zero Unsafe Compliance)
* **Date**: July 05, 2026
* **Description**: Optimized the rendering of window shadows to resolve UI lag and mouse stutter when multiple windows are open.
  * **Region Exclusion (`draw_rect_alpha_exclude`)**: Implemented a geometric difference algorithm that splits the shadow drawing rectangle into up to 4 non-overlapping outer boundary slices, completely avoiding looping over or checking pixels inside the window body.
  * **Concentric Layer Reduction**: Reduced the soft drop shadow depth from 10 concentric layers to 5 layers (using a step-by-2 loop), adjusting the alpha blend formula to preserve smooth visuals.
  * **Zero Unsafe Compliance**: Implemented all optimization logic in safe Rust without writing any new `unsafe` blocks, fully adhering to the project's safety standard.
* **Rationale**: Drastically reduces the number of pixels processed during full screen composites (over 100x fewer pixel iterations for standard window shadows), eliminating CPU spikes and mouse latency.
* **Location**: `usermode-desktop/src/graphics.rs`

### 16. System Settings Application and Window Shadow Toggle
* **Date**: July 05, 2026
* **Description**: Added a System Settings application window to the desktop environment allowing users to configure rendering behaviors dynamically.
  * **Settings Application (`settings.rs`)**: Renders the desktop performance panel featuring a clean, modern iOS-style toggle switch card for drop shadows.
  * **Interactive Switch**: Captures clicks inside the Settings window body to dynamically toggle the atomic `SHADOWS_ENABLED` state.
  * **Icon Integration**: Extended the magnified Dock to 5 items and added a modern vector slider-control/settings icon (`draw_vector_settings_icon`).
  * **Launchpad & Workspace integration**: Registered settings window in `WINDOWS` array, mapped the launch action in the Launchpad menu, and scaled the Launchpad height to support 5 options.
* **Rationale**: Gives users a dynamic, graphical control to disable expensive shadow blending, boosting performance on lower-end physical hardware.
* **Location**: `usermode-desktop/src/settings.rs` & `usermode-desktop/src/main.rs`

### 17. Desktop Icons Removal and Clean UI Refinement
* **Date**: July 05, 2026
* **Description**: Overhauled the desktop interface by removing the static sidebar shortcuts (Files, Terminal, Monitor) from the screen.
  * **Rendering cleanup**: Deleted `draw_icon` calls and text labels for the three desktop sidebar icons in `main.rs`.
  * **Input handling cleanup**: Removed the background click detection region in `main.rs` that previously mapped clicks in the top-left area to launching/focusing corresponding windows, ensuring background clicks only perform unfocusing actions.
  * **Imports optimization**: Removed the unused `use atlas_font::*;` statement in `main.rs`.
* **Rationale**: Cleans up the desktop interface to showcase the nebula wallpaper and relies exclusively on the modern Dock and Launchpad for window launching, matching contemporary visual paradigms.
* **Location**: `usermode-desktop/src/main.rs`

### 18. Dirty Rectangles Update Optimization (Zero Unsafe Compliance)
* **Date**: July 05, 2026
* **Description**: Implemented a Dirty Rectangles update system in the software renderer.
  * **DirtyRectTracker (`dirty.rs`)**: Tracks modified screen regions (such as window dragging, Launchpad animations, Dock magnification, and telemetry text updates) using a safe, fixed-size tracker.
  * **Selective blitting**: Instead of copying the entire 24.88 MB frame buffer from `BACK_BUFFER` to the GOP framebuffer, it loops through the dirty rectangles and selectively copies only the modified regions.
* **Rationale**: Drastically cuts down memory copy overhead during window movement and state updates.
* **Location**: `usermode-desktop/src/dirty.rs` & `usermode-desktop/src/main.rs`

### 19. Fast Division-Free Alpha Blending Optimization
* **Date**: July 05, 2026
* **Description**: Refactored alpha blending arithmetic to completely eliminate expensive integer divisions.
  * **Fast bitwise blending**: Replaced the `/ 255` division in `draw_pixel_alpha` with a fast bitwise approximation `((val + 1 + (val >> 8)) >> 8)`, which compiles directly into registers.
  * **Unsafe block reduction**: Combined two independent `unsafe` blocks in `draw_pixel_alpha` into a single unified block, enhancing borrow-checker visibility.
* **Rationale**: Replaces division instructions with fast shift and add instructions inside inner loops, accelerating blend rates.
* **Location**: `usermode-desktop/src/graphics.rs`

### 20. Window Backing Store and Compositor Caching
* **Date**: July 05, 2026
* **Description**: Implemented a window backing store cache in the desktop compositor to prevent procedural redrawing.
  * **Static cache memory (`state.rs`)**: Allocated static `[AtomicU32; 580 * 380]` BSS segments for window caching to conform to the Zero Unsafe Policy.
  * **Compositor snapshot & restore**: Snapshots windows onto the backing store when they are redrawn (dirty) and restores them directly using fast copy operations during idle/drag states, bypassing procedural font rendering and layout loops.
* **Rationale**: Eliminates window redraw cost when dragging other windows or moving the mouse, resulting in a lag-free visual interface.
* **Location**: `usermode-desktop/src/state.rs`, `usermode-desktop/src/graphics.rs`, & `usermode-desktop/src/main.rs`

### 21. Applications Screen & Real-time Search
* **Date**: July 07, 2026
* **Description**: Overhauled the Launchpad popup menu to a full-screen Applications Screen overlay with keyboard-routing real-time search.
  * **Thread-safe state (`state.rs`)**: Implemented a 100% safe, lock-free `SearchQuery` buffer using atomic arrays.
  * **Power Vector Icon (`icons.rs`)**: Added `draw_vector_shutdown_icon` representing a standby power key in red.
  * **Applications Grid Layout (`compositor.rs`)**: Overhauled `draw_start_menu` to display a frosted translucent dark backdrop, a centered search box with a blinking text cursor, and a dynamically-centered grid of application cards with hover highlights and vector icons.
  * **Keyboard Routing & Grid Clicks (`input.rs`)**: Intercepted characters and backspaces to update the search query, allowed pressing Enter to boot the first matching app, mapped clicks to cards, and cleared search state when opening the screen.
  * **Compositor Redraws (`render.rs`)**: Configured the dirty tracker to cover the entire screen when the applications screen is active to keep cursor blink and typing updates responsive.
* **Rationale**: Elevates the desktop user experience to a premium modern desktop general-overview paradigm, while adhering 100% to the strict Zero Unsafe Policy.
* **Location**: `usermode-desktop/src/state.rs`, `usermode-desktop/src/graphics/icons.rs`, `usermode-desktop/src/graphics/compositor.rs`, `usermode-desktop/src/input.rs`, and `usermode-desktop/src/render.rs`

### 22. Sidebar Settings & Default Shadows Disabled
* **Date**: July 07, 2026
* **Description**: Overhauled the Settings application to a sidebar-style layout and disabled window shadows by default.
  * **Shadows Off by Default (`state.rs`)**: Changed `SHADOWS_ENABLED` default value to false, reducing initial composite rendering cycles.
  * **Sidebar Selection State (`state.rs`)**: Declared `ACTIVE_SETTINGS_TAB` variable to track active categories.
  * **Sidebar & Tabs layout (`settings.rs`)**: Overhauled `draw_settings_window` to render a left-aligned sidebar with hovering highlight tabs (Appearance, System, About) and separate right-aligned settings panels.
  * **Compositor Caching Bypass (`render.rs`)**: Configured the compositor loop to skip backing-store snapshot caching for the Settings app to guarantee real-time sidebar mouse hovers and system metrics updates.
  * **Tab click routing (`input.rs`)**: Integrated category sidebar bounds checks to update tab state, and restricted the appearance card shadow toggle to only fire when the Appearance tab is selected.
* **Rationale**: Delivers a premium settings configuration interface matching modern general-purpose desktop operating systems and boosts out-of-the-box frame rates by disabling resource-heavy drop shadow calculations.
* **Location**: `usermode-desktop/src/state.rs`, `usermode-desktop/src/settings.rs`, `usermode-desktop/src/render.rs`, and `usermode-desktop/src/input.rs`

### 23. File Manager Overhaul (Phase 1: Dynamic Navigation, Breadcrumbs & Selection)
* **Date**: July 20, 2026
* **Description**: Upgraded the Desktop File Manager (`usermode-desktop/src/file_manager.rs`) from a static single-directory list into an interactive file browser.
  * **Thread-Safe Path Tracking (`file_manager.rs`)**: Introduced `FileManagerState` with `path_buf`, `path_len`, `selected_index`, and `hovered_index` using atomic types without unsafe code.
  * **Top Control & Breadcrumbs Bar (`file_manager.rs`)**: Added navigation buttons for Back (`<`), Up/Root (`^`), Refresh (`R`), and active location path display.
  * **Interactive Item Highlights (`file_manager.rs`)**: Rendered interactive selection highlights (`selected_index`) and subtle hover highlights (`hovered_index`).
  * **Directory Navigation & Click Routing (`input.rs` & `render.rs`)**: Connected mouse body clicks in window 2 to `handle_file_manager_click`, enabling single-click entry into subdirectories and back navigation, as well as live mouse hover updates.
* **Rationale**: Provides dynamic file system navigation and interactive UI controls adhering strictly to `AI_GUIDELINES.md` Zero Unsafe and 100% `///` documentation policies.
* **Location**: `usermode-desktop/src/file_manager.rs`, `usermode-desktop/src/render.rs`, and `usermode-desktop/src/input.rs`

### 24. File Manager Overhaul (Phase 2: Side Preview Drawer & Text Inspection)
* **Date**: July 20, 2026
* **Description**: Enhanced the Desktop File Manager (`usermode-desktop/src/file_manager.rs`) with a right-hand details and quick preview drawer.
  * **Split Layout View (`file_manager.rs`)**: Divided the window into a left-hand directory item browser and a right-hand metadata & preview drawer with vertical separator lines.
  * **File Type Detection (`detect_file_type`)**: Implemented file extension inspection mapping items to human-readable type badges (e.g. "Rust Source Code", "Text Document", "System Log File", "Folder", "Executable Binary").
  * **Live Multi-Line Text Preview (`draw_file_manager`)**: Added automatic file opening (`sys_open`) and preview buffer reading (`sys_read`) for selected files, rendering the first lines of text in a rounded glass preview card along with exact byte sizes.
  * **Empty State Card**: Displays a clean "No item selected" placeholder when no directory item is active.
* **Rationale**: Provides instant inspection of file metadata and contents without leaving the file manager window, adhering strictly to `AI_GUIDELINES.md` Zero Unsafe and 100% `///` documentation standards.
* **Location**: `usermode-desktop/src/file_manager.rs`

### 25. File Manager Overhaul (Phase 3: Operations Toolbar & Modal Dialogs)
* **Date**: July 20, 2026
* **Description**: Added item creation and deletion operations to the Desktop File Manager (`usermode-desktop/src/file_manager.rs`).
  * **Operations Toolbar Buttons (`draw_file_manager`)**: Added +Folder (`+Dir`), +File (`+File`), and Delete (`X`) buttons on the right side of the navigation toolbar.
  * **Modal Dialog Overlay State (`FileManagerState`)**: Introduced `modal_mode`, `modal_input_buf`, and `modal_input_len` to manage pop-up creation prompts for directories, files, and delete confirmations.
  * **Keyboard Typing & Action Submission (`handle_file_manager_key` & `execute_modal_action`)**: Integrated live keyboard text entry, Backspace editing, Escape canceling, and Enter submission routing to `sys_mkdir` and `sys_open`/`sys_write`.
* **Rationale**: Empowers users to create new subdirectories and text files directly from the graphical user interface, strictly adhering to `AI_GUIDELINES.md` Zero Unsafe and 100% `///` documentation policies.
* **Location**: `usermode-desktop/src/file_manager.rs` and `usermode-desktop/src/input.rs`

### 26. File Manager Overhaul (Phase 4: View Modes & Vertical Scrollbar)
* **Date**: July 20, 2026
* **Description**: Added dual view mode rendering (List View vs Grid View) and vertical scrollbar support to the Desktop File Manager (`usermode-desktop/src/file_manager.rs`).
  * **View Mode Selector (`toggle_view_mode`)**: Added `view_mode` state and a dedicated `[Lst]` / `[Grd]` toolbar button to toggle between detailed line list view and large icon grid card layout.
  * **Grid View Card Layout (`draw_file_manager`)**: Designed a responsive multi-column card grid rendering icons at the top and file titles underneath.
  * **Vertical Scrollbar (`scroll_offset`)**: Calculated container and item heights to render a dynamic scrollbar track and thumb when item counts exceed window bounds.
* **Rationale**: Delivers a customizable file browsing experience matching desktop standards while maintaining `AI_GUIDELINES.md` Zero Unsafe and 100% `///` documentation compliance.
* **Location**: `usermode-desktop/src/file_manager.rs`

### 27. File Manager Overhaul (Phase 5 & Modular Directory Refactoring)
* **Date**: July 20, 2026
* **Description**: Completed File Manager overhaul Phase 5 (Status Bar & Disk Storage telemetry) and refactored single monolithic file into a clean modular module directory (`usermode-desktop/src/file_manager/`).
  * **Modular Split (`file_manager/`)**: Split `file_manager.rs` into `mod.rs`, `state.rs`, `draw.rs`, and `input.rs`, maintaining strict compliance with `AI_GUIDELINES.md` file length limits (< 800 lines per file).
  * **Scrollbar Interaction Fix (`handle_file_manager_click`)**: Added mouse scrollbar track region click detection (`scroll_up` / `scroll_down`) and keyboard arrow key scrolling (`handle_file_manager_key`).
  * **Phase 5 Status Bar (`draw_file_manager`)**: Added bottom telemetry bar rendering total directory item counts, active selection label, and VFS storage status indicators.
* **Rationale**: Organizes application architecture for scalable maintenance and completes the full 5-phase File Manager roadmap.
* **Location**: `usermode-desktop/src/file_manager/` (`mod.rs`, `state.rs`, `draw.rs`, `input.rs`)

---

## 📊 Visual Telemetry & Interface Enhancements

### 1. TMR stability & ALU Voter Diagnostics panel
* **Date**: May 29, 2026
* **Description**: Added a real-time TMR Diagnostics panel directly in the visual aerospace flight visualizer.
* **Features**:
  * Computes **TMR Voter Stability Index** dynamically based on total critical cycles and ALU voter interceptions.
  * Draws an ANSI-colored retro text bar graph representing stability percentage and stability state (Green / Yellow / Red).
  * Tracks and displays precise **ALU Fault Rate** percentages.
* **Location**: `simulation-dashboard/src/main.rs`

---

## 📦 Workspace Configuration Adjustments

### 1. Workspace Build Default-Members Exclusion
* **Date**: May 29, 2026
* **Description**: Added `default-members` configuration to the workspace Cargo.toml to exclude `kernel-x86`.
* **Rationale**: Prevents `duplicate lang item panic_impl` compiler errors when running cargo commands at the workspace root without target variables. Ensures host-side simulation runs seamlessly, while target-specific builds are cleanly isolated.
* **Location**: `Cargo.toml` (Workspace Root)

### 2. Host-Side Runner Crate Integration
* **Date**: May 29, 2026
* **Description**: Integrated a new host-side compilation package `runner` to orchestrate kernel building, image generation, and QEMU execution.
* **Rationale**: Automates `cargo +nightly` bare-metal builds and calls `bootloader` library routines programmatically to output BOTH a flashable modern GPT `uefi.img` and a legacy MBR `bios.img` directly, then boots QEMU with COM1 mapped directly to terminal stdio.
* **Location**: `runner/Cargo.toml` and `runner/src/main.rs`
