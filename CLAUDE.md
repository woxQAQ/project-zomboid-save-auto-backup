# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Start

First-time setup:
```bash
pnpm install      # Install dependencies
pnpm tauri dev    # Start development server
```

## Project Overview

A **Tauri + React + TypeScript** desktop application for **Project Zomboid save game backup/restore**. Provides automated backups and restore functionality (Save/Load) for the hardcore survival game Project Zomboid.

**Tech Stack:**
- **Backend**: Rust (Tauri 2.x)
- **Frontend**: React 19 + TypeScript + Vite
- **Package Manager**: pnpm (use pnpm, not npm/yarn)
- **Development Environment**: Nix flake (optional, for reproducible dev shell)

## Development Commands

```bash
# Install dependencies
pnpm install

# Development mode (runs Tauri app with hot reload)
pnpm tauri dev

# Build for production
pnpm tauri build

# Frontend-only development (rarely used)
pnpm dev

# Type checking
pnpm tsc --noEmit

# Rust backend commands
pnpm tauri:check      # Cargo clippy (linting)
pnpm tauri:test       # Cargo test (run tests)
pnpm tauri:build:check # Cargo check (compile check)

# Frontend linting
pnpm lint          # Biome check

# Fix linting issues
pnpm lint:fix
```

**Note**: Always use `pnpm tauri dev` for development - this builds both the frontend and Rust backend together.

## Architecture

### Directory Structure
```
src/              # React frontend (TypeScript)
src-tauri/        # Rust backend (Tauri)
src-tauri/src/
  ├── main.rs     # Tauri entry point
  └── lib.rs      # Tauri commands (exposed to frontend)
docs/             # Product and UI documentation
public/           # Static assets
```

### Tauri Commands Pattern
Rust functions are exposed to frontend via Tauri commands in `src-tauri/src/lib.rs`. Commands are decorated with `#[tauri::command]` and must return `Result<T, E>` for proper error handling to the frontend.

### Communication Pattern
- Frontend calls Rust commands via `invoke()` from `@tauri-apps/api/core`
- Async commands use promises
- File system operations require proper capabilities in `src-tauri/capabilities/default.json`

## Key Application Concepts

### Save Paths (Platform-Specific)
Project Zomboid stores saves in:
- **Windows**: `C:\Users\<User>\Zomboid\Saves`
- **Mac/Linux**: `~/Zomboid/Saves`

The app must auto-detect these paths and allow manual override.

### Backup Strategy
- **Format**: `{SaveName}_{YYYY-MM-DD}_{HH-mm-ss}`
- **GC (Garbage Collection)**: Auto-delete old backups exceeding retention limit (default: 10)
- **Pre-restore safety**: Before ANY restore operation, create an "Undo snapshot" of current save

### UI/UX Philosophy
- **Dark theme**: Colors inspired by Project Zomboid (black, gray, red accents)
- **Minimal layout**: Single-page dashboard with settings modal
- **Safety first**: All destructive operations (restore/delete) require confirmation
- See `docs/ui_design.md` for full visual specifications

## Feature Dependencies

The project has a structured feature list in `FEATURE_LIST.toon`. Key dependencies:
- **CORE-01** (basic file operations) must be implemented before any other CORE features
- **CORE-02** (path detection + config) enables UI-02 (settings) and UI-03 (dashboard)
- **CORE-04** (safe restore) requires CORE-01 and CORE-03
- UI features depend on their corresponding CORE features

Implement features in dependency order for smooth development.

## Configuration Files

- **`src-tauri/tauri.conf.json`**: Tauri app configuration (window size, build commands)
- **`src-tauri/capabilities/default.json`**: Permissions for Tauri commands (needs file system access)
- **`vite.config.ts`**: Frontend build config
- **`tsconfig.json`**: TypeScript configuration

## Documentation

- **`docs/prd.md`**: Product Requirements Document - complete feature specs, user personas, roadmap
- **`docs/ui_design.md`**: Detailed UI/UX specifications, color palette, component designs, interaction flows

## Important Implementation Notes

1. **Async I/O**: All file operations must use async/await to avoid blocking the UI thread
2. **Error Handling**: Never fail silently - always show clear error messages for file operation failures
3. **Memory**: Keep memory usage under 100MB (avoid loading entire saves into memory)
4. **File Locking**: Project Zomboid may lock files while running - detect and handle gracefully
