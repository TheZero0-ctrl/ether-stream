import { invoke } from "@tauri-apps/api/core";
import type { BootstrapState } from "../types/bootstrap";

export function getBootstrapState() {
  return invoke<BootstrapState>("get_bootstrap_state");
}
