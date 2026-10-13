import { invoke } from "@tauri-apps/api/core";

export type RoutineKind = "daily" | "weekly";

export interface RoutineItemDto {
  id: string;
  kind: RoutineKind | string;
  title: string;
  isBuiltin: boolean;
  ignored: boolean;
  done: boolean;
  sortOrder: number;
}

export interface CreateRoutine {
  kind: RoutineKind;
  title: string;
}

export interface UpdateRoutine {
  id: string;
  title?: string;
  done?: boolean;
  ignored?: boolean;
}

export interface ReorderRoutines {
  kind: RoutineKind;
  ids: string[];
}

export function listRoutines() {
  return invoke<RoutineItemDto[]>("list_routines");
}

export function createRoutine(input: CreateRoutine) {
  return invoke<RoutineItemDto>("create_routine", { input });
}

export function updateRoutine(patch: UpdateRoutine) {
  return invoke<RoutineItemDto>("update_routine", { patch });
}

export function deleteRoutine(id: string) {
  return invoke<void>("delete_routine", { id });
}

export function reorderRoutines(input: ReorderRoutines) {
  return invoke<void>("reorder_routines", { input });
}
