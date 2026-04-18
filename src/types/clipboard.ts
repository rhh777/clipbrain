/** 内容类型 — 与 Rust 后端 ContentType 对应 */
export type ContentType =
  | { type: "Json" }
  | { type: "Yaml" }
  | { type: "Url" }
  | { type: "Email" }
  | { type: "PhoneNumber" }
  | { type: "IdCard" }
  | { type: "MathExpression" }
  | { type: "Code"; detail: string }
  | { type: "TableData"; detail: string }
  | { type: "FileList" }
  | { type: "PlainText" }
  | { type: "Unknown" };

/** 操作描述 */
export interface ActionDescriptor {
  id: string;
  display_name: string;
  description: string;
  action_scope: "general" | "specific";
  requires_model: boolean;
  estimated_duration_ms: number;
}

/** 操作执行结果 */
export interface ActionOutput {
  result: string;
  result_type: string;
}

export interface ClipboardHistoryEventItem {
  id: number;
  content: string | null;
  image_path: string | null;
  content_type: string;
  source_app: string | null;
  char_count: number | null;
  created_at: string;
  is_pinned: boolean;
  is_sensitive: boolean;
}

/** 剪贴板变化事件 payload */
export interface ClipboardChangeEvent {
  content: string;
  content_type: ContentType;
  preview: string;
  actions: ActionDescriptor[];
  timestamp: number;
  item?: ClipboardHistoryEventItem | null;
}

/** 流式操作事件 payload */
export interface ActionStreamPayload {
  event_type: "start" | "thinking" | "delta" | "done" | "error";
  action_id: string;
  content: string;
  result_type: string;
}

/** 获取内容类型的显示标签 */
export function contentTypeLabel(ct: ContentType): string {
  const labels: Record<string, string> = {
    Json: "JSON",
    Yaml: "YAML",
    Url: "URL",
    Email: "邮箱",
    PhoneNumber: "手机号",
    IdCard: "身份证",
    MathExpression: "数学表达式",
    Code: "代码",
    TableData: "表格",
    FileList: "文件",
    PlainText: "文本",
    Unknown: "未知",
  };
  return labels[ct.type] ?? ct.type;
}

/** 获取内容类型的颜色 */
export function contentTypeColor(ct: ContentType): string {
  const colors: Record<string, string> = {
    Json: "bg-yellow-500/20 text-yellow-400",
    Yaml: "bg-orange-500/20 text-orange-400",
    Url: "bg-blue-500/20 text-blue-400",
    Email: "bg-cyan-500/20 text-cyan-400",
    PhoneNumber: "bg-green-500/20 text-green-400",
    IdCard: "bg-red-500/20 text-red-400",
    MathExpression: "bg-purple-500/20 text-purple-400",
    Code: "bg-emerald-500/20 text-emerald-400",
    TableData: "bg-teal-500/20 text-teal-400",
    FileList: "bg-indigo-500/20 text-indigo-400",
    PlainText: "bg-gray-500/20 text-gray-400",
    Unknown: "bg-gray-500/20 text-gray-400",
  };
  return colors[ct.type] ?? colors["Unknown"];
}
