import { invoke } from "@tauri-apps/api/core";

export interface UserTodoDto {
  id: string;
  body: string;
  done: boolean;
  sortOrder: number;
}

export interface UpdateUserTodo {
  id: string;
  body?: string;
  done?: boolean;
  sortOrder?: number;
}

export function listUserTodos() {
  return invoke<UserTodoDto[]>("list_user_todos");
}

export function createUserTodo(body: string) {
  return invoke<UserTodoDto>("create_user_todo", { body });
}

export function updateUserTodo(patch: UpdateUserTodo) {
  return invoke<UserTodoDto>("update_user_todo", { patch });
}

export function deleteUserTodo(id: string) {
  return invoke<void>("delete_user_todo", { id });
}

export interface ReorderUserTodos {
  ids: string[];
}

export function reorderUserTodos(input: ReorderUserTodos) {
  return invoke<void>("reorder_user_todos", { input });
}
