// Paired editorial copy. Product identity: Claude Code environment and data manager.
export const groups = [
  {
    "id": "overview",
    "no": "01",
    "label": {
      "zh": "产品概览",
      "en": "PRODUCT OVERVIEW"
    }
  },
  {
    "id": "work",
    "no": "02",
    "label": {
      "zh": "工作资料",
      "en": "WORK DATA"
    }
  },
  {
    "id": "settings",
    "no": "03",
    "label": {
      "zh": "环境控制",
      "en": "ENVIRONMENT CONTROL"
    }
  }
];

export const cards = [
  {
    "id": "overview-01",
    "group": "overview",
    "page": 1,
    "kind": "cover",
    "eyebrow": {
      "zh": "LINTEL · 0.1.0 开发者 PREVIEW",
      "en": "LINTEL · 0.1.0 DEVELOPER PREVIEW"
    },
    "title": {
      "zh": "Claude Code 环境与资料管理器",
      "en": "Claude Code environment & data manager"
    },
    "lead": {
      "zh": "管理配置、工作资料与本地状态。支持本机与 SSH 远端环境。",
      "en": "Configuration, work data and local state. On your Mac and over SSH."
    },
    "body": {
      "zh": "在 App 里预览与操作，也可使用 CLI 或 Agent。",
      "en": "Review and act in the app, or use the CLI and an agent."
    },
    "media": "native-home"
  },
  {
    "id": "overview-02",
    "group": "overview",
    "page": 2,
    "kind": "map",
    "eyebrow": {
      "zh": "环境与资料管理",
      "en": "ENVIRONMENT & DATA MANAGEMENT"
    },
    "title": {
      "zh": "管理范围",
      "en": "What Lintel manages"
    },
    "lead": {
      "zh": "环境、资料与日常维护，都在这里。",
      "en": "Environments, data and everyday maintenance."
    },
    "items": [
      {
        "icon": "tune",
        "title": {
          "zh": "环境与配置",
          "en": "Environments & configuration"
        },
        "text": {
          "zh": "登记、创建、启动、编辑与复制。",
          "en": "Register, create, launch, edit and copy."
        }
      },
      {
        "icon": "folder",
        "title": {
          "zh": "工作与会话",
          "en": "Work & sessions"
        },
        "text": {
          "zh": "选择、归档、阅读、迁入与交接。",
          "en": "Select, archive, read, import and hand off."
        }
      },
      {
        "icon": "broom",
        "title": {
          "zh": "本地状态",
          "en": "Local state"
        },
        "text": {
          "zh": "残留、客户端标识、登录修复与重建。",
          "en": "Residue, client identifier, login repair and rebuild."
        }
      },
      {
        "icon": "receipt",
        "title": {
          "zh": "变化与恢复",
          "en": "Changes & recovery"
        },
        "text": {
          "zh": "快照对照、漂移检查、按字段恢复。",
          "en": "Snapshots, drift checks and field-aware restore."
        }
      },
      {
        "icon": "panels",
        "title": {
          "zh": "浏览器与网络",
          "en": "Browser & network"
        },
        "text": {
          "zh": "Profile、遥测设置、受控通道与出口。",
          "en": "Profiles, telemetry settings, channels and egress."
        }
      },
      {
        "icon": "terminal",
        "title": {
          "zh": "远端与 Agent",
          "en": "Remote & agents"
        },
        "text": {
          "zh": "SSH、Linux runner、CLI 与原任务。",
          "en": "SSH, Linux runner, CLI and original tasks."
        }
      }
    ],
    "footer": {
      "zh": "每次修改先看清目标与范围，再批准执行；完成后核对原任务。",
      "en": "Review the target and scope before approving a change. Check its original task afterward."
    }
  },
  {
    "id": "overview-03",
    "group": "overview",
    "page": 3,
    "kind": "work-overview",
    "eyebrow": {
      "zh": "工作资料 · 选择与阅读",
      "en": "WORK DATA · SELECT & READ"
    },
    "title": {
      "zh": "指令、记忆与会话资料",
      "en": "Instructions, memory & sessions"
    },
    "lead": {
      "zh": "选取指令、记忆和会话，收进加密工作包；随后可以阅读、迁入或整理交接稿。",
      "en": "Choose instructions, memory and sessions for an encrypted package."
    },
    "media": "native-archive-selection",
    "steps": [
      {
        "label": {
          "zh": "选择工作资料",
          "en": "Select work data"
        },
        "detail": {
          "zh": "仅选中内容进入工作包",
          "en": "Only selected content enters the package"
        }
      },
      {
        "label": {
          "zh": "独立阅读",
          "en": "Read independently"
        },
        "detail": {
          "zh": "不依赖来源任务记录",
          "en": "No source-job record required"
        }
      },
      {
        "label": {
          "zh": "迁入另作决定",
          "en": "Import separately"
        },
        "detail": {
          "zh": "选择目标，再次预览批准",
          "en": "Choose a target and approve again"
        }
      }
    ],
    "note": {
      "zh": "原件保留在原处。配置与凭据使用各自的管理入口。",
      "en": "Originals stay in place. Configuration and credentials have separate management paths."
    }
  },
  {
    "id": "overview-04",
    "group": "overview",
    "page": 4,
    "kind": "local-state",
    "eyebrow": {
      "zh": "本地状态与记录",
      "en": "LOCAL STATE & RECORDS"
    },
    "title": {
      "zh": "本地状态与变化记录",
      "en": "Local state and recorded changes"
    },
    "lead": {
      "zh": "查看具体残留与本地标识，对照环境变化，按原任务恢复配置字段。",
      "en": "Inspect residue and local identifiers, compare changes, and restore configuration fields from original tasks."
    },
    "stateItems": [
      {
        "title": {
          "zh": "残留归档与清理",
          "en": "Residue backup and clear"
        },
        "text": {
          "zh": "只清理已识别的对象；可先写独立加密残留备份。",
          "en": "Clear identified objects only, optionally after an independent encrypted residue backup."
        }
      },
      {
        "title": {
          "zh": "本地客户端标识",
          "en": "Local client identifier"
        },
        "text": {
          "zh": "明确移除本地 userID，保留其他键；不注销、不清凭据。",
          "en": "Explicitly remove local userID while keeping other keys; no logout or credential clearing."
        }
      },
      {
        "title": {
          "zh": "快照与配置字段恢复",
          "en": "Snapshots and config-field restore"
        },
        "text": {
          "zh": "比较语义快照；原任务只恢复该任务拥有的配置字段。",
          "en": "Compare semantic snapshots; original tasks restore only their owned configuration fields."
        }
      }
    ],
    "note": {
      "zh": "本地标识重置不会解绑账号；字段恢复会保留后来编辑。",
      "en": "Local identifier reset does not unlink accounts. Field restore preserves later edits."
    }
  },
  {
    "id": "overview-05",
    "group": "overview",
    "page": 5,
    "kind": "surfaces",
    "eyebrow": {
      "zh": "本机 · 远端 · 自动化入口",
      "en": "LOCAL · REMOTE · AUTOMATION"
    },
    "title": {
      "zh": "macOS App、CLI 与 SSH",
      "en": "macOS app, CLI and SSH"
    },
    "lead": {
      "zh": "在 Mac 上操作本机环境，也能通过 SSH 连接 Linux，把明确任务交给 CLI 或 Agent。",
      "en": "Manage local environments, connect over SSH, or use the CLI and agents."
    },
    "terminal": {
      "title": {
        "zh": "独立 CLI",
        "en": "Standalone CLI"
      },
      "lines": [
        "$ lintel --help",
        "$ lintel capabilities --json",
        "$ lintel describe --json"
      ]
    },
    "panels": [
      {
        "title": {
          "zh": "macOS App",
          "en": "macOS app"
        },
        "text": {
          "zh": "本机环境与资料操作，先预览再批准。",
          "en": "Local environments and data, reviewed before approval."
        }
      },
      {
        "title": {
          "zh": "CLI 与 Agent",
          "en": "CLI & agents"
        },
        "text": {
          "zh": "同一 core 的终端入口；交接明确原 ID 与查询命令。",
          "en": "Terminal entry to the same core, with explicit IDs and query commands."
        }
      },
      {
        "title": {
          "zh": "SSH 目标主机",
          "en": "SSH target host"
        },
        "text": {
          "zh": "在远端核对环境与 runner；断开后查询原任务。",
          "en": "Verify environments and runners remotely; query original tasks after disconnecting."
        }
      }
    ],
    "note": {
      "zh": "Linux CLI／runner 为实验性。断线后查询原任务，不重复提交。",
      "en": "Linux CLI/runner support is experimental. Query the original task after disconnecting; do not resubmit."
    }
  },
  {
    "id": "overview-06",
    "group": "overview",
    "page": 6,
    "kind": "closing",
    "eyebrow": {
      "zh": "版本与平台",
      "en": "VERSION & PLATFORMS"
    },
    "title": {
      "zh": "0.1.0 开发者 Preview",
      "en": "0.1.0 developer Preview"
    },
    "lead": {
      "zh": "App、CLI 与文档已经发布，欢迎按安装说明试用。",
      "en": "The app, CLI and documentation are available. Read the installation notes before trying them."
    },
    "facts": [
      {
        "title": {
          "zh": "macOS App",
          "en": "macOS app"
        },
        "text": {
          "zh": "Apple Silicon · ad-hoc 签名",
          "en": "Apple Silicon · ad-hoc signed"
        }
      },
      {
        "title": {
          "zh": "独立 CLI",
          "en": "Standalone CLI"
        },
        "text": {
          "zh": "macOS arm64 · Linux x86_64／aarch64",
          "en": "macOS arm64 · Linux x86_64 / aarch64"
        }
      },
      {
        "title": {
          "zh": "更新方式",
          "en": "Updates"
        },
        "text": {
          "zh": "手动下载；没有启用的 updater feed",
          "en": "Manual download; no active updater feed"
        }
      },
      {
        "title": {
          "zh": "Apple 信任",
          "en": "Apple trust"
        },
        "text": {
          "zh": "无 Developer ID 签名或公证",
          "en": "No Developer ID signing or notarization"
        }
      }
    ],
    "notice": {
      "zh": "无 Developer ID 签名／Apple 公证；干净 Mac 首装未验证，可能被系统阻止。Linux 为实验性。",
      "en": "No Developer ID signing or Apple notarization. First install on a clean Mac is unverified and may be blocked. Linux is experimental."
    }
  },
  {
    "id": "work-01",
    "group": "work",
    "page": 1,
    "kind": "work-cover",
    "eyebrow": {
      "zh": "工作资料 · 02",
      "en": "WORK DATA · 02"
    },
    "title": {
      "zh": "工作资料管理",
      "en": "Managing work data"
    },
    "lead": {
      "zh": "备份、阅读和迁移 Claude Code 的指令、记忆与会话。",
      "en": "Preserve, read and transfer Claude Code instructions, memory and sessions."
    },
    "body": {
      "zh": "工作资料管理是 Lintel 的一部分；配置、本地状态、浏览器与远端环境也有各自入口。",
      "en": "Work data is one part of Lintel, alongside configuration, local state, browsers and remote environments."
    }
  },
  {
    "id": "work-02",
    "group": "work",
    "page": 2,
    "kind": "work-selection",
    "eyebrow": {
      "zh": "元数据清单 · 内容选择",
      "en": "METADATA INVENTORY · CONTENT SELECTION"
    },
    "title": {
      "zh": "按文件选择",
      "en": "Select exact files"
    },
    "lead": {
      "zh": "先查看文件清单，再决定哪些指令、记忆和会话进入工作包。",
      "en": "Inspect the file listing, then choose which instructions, memory and sessions enter the package."
    },
    "media": "native-archive-selection",
    "callout": {
      "zh": "只归档选中的内容，原件留在原处。",
      "en": "Archive selected contents only. Originals stay in place."
    },
    "note": {
      "zh": "清单读取文件元数据。凭据、settings、hooks、MCP 与插件不包含在工作包里。",
      "en": "The listing reads file metadata. Credentials, settings, hooks, MCP and plugins are outside this work package."
    }
  },
  {
    "id": "work-03",
    "group": "work",
    "page": 3,
    "kind": "work-package",
    "eyebrow": {
      "zh": "审阅 · 批准 · 加密",
      "en": "REVIEW · APPROVE · ENCRYPT"
    },
    "title": {
      "zh": "加密归档",
      "en": "Encrypted archives"
    },
    "lead": {
      "zh": "核对选中的文件与目标，用自己的口令生成 .age 加密工作包。Lintel 不保存口令。",
      "en": "Review selected files and the destination, then create a .age package with your passphrase. Lintel does not store it."
    },
    "steps": [
      {
        "label": {
          "zh": "已选工作原件",
          "en": "Selected original files"
        },
        "detail": {
          "zh": "内容与目标冻结",
          "en": "Content and target frozen"
        }
      },
      {
        "label": {
          "zh": "预览并批准",
          "en": "Preview and approve"
        },
        "detail": {
          "zh": "一个原任务，一次决定",
          "en": "One original task, one decision"
        }
      },
      {
        "label": {
          "zh": "加密工作包",
          "en": "Encrypted work package"
        },
        "detail": {
          "zh": ".age · 至少 12 字符口令",
          "en": ".age · at least 12-character passphrase"
        }
      }
    ],
    "note": {
      "zh": "归档只生成工作包，不创建环境、不迁移账号。其他备份格式另有说明。",
      "en": "Archiving creates a work package, not an environment or account transfer. Other backup formats have their own guides."
    }
  },
  {
    "id": "work-04",
    "group": "work",
    "page": 4,
    "kind": "work-reader",
    "eyebrow": {
      "zh": "独立加密文件入口",
      "en": "INDEPENDENT ENCRYPTED-FILE ENTRY"
    },
    "title": {
      "zh": "独立阅读工作包",
      "en": "Read a work package independently"
    },
    "lead": {
      "zh": "打开独立的加密工作包，查看文件、结构化消息和原始文本。",
      "en": "Open an encrypted package independently to inspect files, structured messages and raw text."
    },
    "media": "native-reader",
    "facts": [
      {
        "zh": "输入口令；不依赖来源任务记录",
        "en": "Enter your passphrase; no source-job record required"
      },
      {
        "zh": "每次读取验证完整包内容",
        "en": "Each read authenticates the complete package"
      },
      {
        "zh": "可阅读的会话不一定支持原生续聊",
        "en": "A readable session may not support native resume"
      }
    ]
  },
  {
    "id": "work-05",
    "group": "work",
    "page": 5,
    "kind": "work-import",
    "eyebrow": {
      "zh": "迁入 · 交接 · 续聊",
      "en": "IMPORT · HANDOFF · RESUME"
    },
    "title": {
      "zh": "迁入与继续工作",
      "en": "Import and continue work"
    },
    "lead": {
      "zh": "阅读之后，再决定把资料放到哪里，以及如何继续工作。",
      "en": "After reading, choose where the data should go and how to continue."
    },
    "branch": {
      "from": {
        "zh": "已审阅工作包",
        "en": "Reviewed package"
      },
      "paths": [
        {
          "label": {
            "zh": "阅读 · 不修改目标",
            "en": "Read · no target changes"
          },
          "state": "read"
        },
        {
          "label": {
            "zh": "迁入 · 冻结最终落点",
            "en": "Import · freeze final destinations"
          },
          "state": "branch"
        },
        {
          "label": {
            "zh": "交接 · 选定片段进入内存",
            "en": "Handoff · selected excerpts in memory"
          },
          "state": "handoff"
        },
        {
          "label": {
            "zh": "原生续聊 · 格式与认证另核对",
            "en": "Native resume · format and authentication checked separately"
          },
          "state": "resume"
        }
      ]
    },
    "callout": {
      "zh": "保存会话资料不等于原生续聊可用；迁入也不会自动启动 Claude。",
      "en": "Preserving session data does not make native resume available; import does not launch Claude automatically."
    },
    "note": {
      "zh": "迁入默认作为参考资料，不覆盖同名内容。原生续聊另受格式、版本与认证条件限制。",
      "en": "Import defaults to reference material without overwriting same-name content. Native resume has separate format, version and authentication requirements."
    }
  },
  {
    "id": "settings-01",
    "group": "settings",
    "page": 1,
    "kind": "settings-cover",
    "eyebrow": {
      "zh": "环境配置 · 本地状态",
      "en": "ENVIRONMENT CONFIG · LOCAL STATE"
    },
    "title": {
      "zh": "环境配置与本地状态",
      "en": "Environment configuration and local state"
    },
    "lead": {
      "zh": "调整已识别设置，编辑或复制配置，处理具体残留与本地客户端标识。",
      "en": "Adjust recognized settings, edit or copy configuration, and handle specific residue and the local client identifier."
    },
    "examples": [
      {
        "key": "policy · 27 fields",
        "value": {
          "zh": "逐项或批量选择",
          "en": "individual or batch selection"
        }
      },
      {
        "key": "user config",
        "value": {
          "zh": "编辑、复制、加密备份",
          "en": "edit, copy, encrypted backup"
        }
      },
      {
        "key": "residue · userID",
        "value": {
          "zh": "精确对象；本地 userID",
          "en": "exact objects; local userID"
        }
      }
    ],
    "note": {
      "zh": "本地 userID 重置不会注销服务端会话，也不会解绑账号。",
      "en": "Resetting local userID does not log out server sessions or unlink accounts."
    }
  },
  {
    "id": "settings-02",
    "group": "settings",
    "page": 2,
    "kind": "settings-actions",
    "eyebrow": {
      "zh": "v5 · 27 个已识别字段",
      "en": "V5 · 27 RECOGNIZED FIELDS"
    },
    "title": {
      "zh": "逐项与批量设置",
      "en": "Individual and batch settings"
    },
    "lead": {
      "zh": "按当前值选择保留、关闭或移除覆盖；也可以批量调整适用字段。",
      "en": "Keep, disable or remove an override based on its current value. Apply suitable actions to several fields at once."
    },
    "actions": [
      {
        "name": {
          "zh": "保留",
          "en": "Keep"
        },
        "code": "keep",
        "text": {
          "zh": "原字段与原始字符串不动。",
          "en": "Leave the original field and string untouched."
        }
      },
      {
        "name": {
          "zh": "关闭",
          "en": "Disable"
        },
        "code": "1",
        "text": {
          "zh": "按该字段已核验的关闭语义写入。",
          "en": "Write the field’s verified off value."
        }
      },
      {
        "name": {
          "zh": "移除本环境覆盖",
          "en": "Remove local override"
        },
        "code": "remove",
        "text": {
          "zh": "只删当前配置根的 user settings 字段。",
          "en": "Delete only this configuration root’s user-settings key."
        }
      }
    ],
    "note": {
      "zh": "移除覆盖后，shell、项目、组织或默认值可能生效。规则或客户端版本变化后，需要重新预览。",
      "en": "Removing an override may expose shell, project, organization or default values. Preview again after a rule or client-version change."
    }
  },
  {
    "id": "settings-03",
    "group": "settings",
    "page": 3,
    "kind": "settings-diff",
    "eyebrow": {
      "zh": "示意 · 合成目标与字段",
      "en": "ILLUSTRATIVE · SYNTHETIC TARGET & FIELDS"
    },
    "title": {
      "zh": "预览目标与字段变化",
      "en": "Preview the target and field changes"
    },
    "lead": {
      "zh": "修改前核对主机、配置根、原值和新值，批准当前这份计划。",
      "en": "Before a change, review the host, configuration root, old and new values, then approve this plan."
    },
    "target": {
      "zh": "配置根（合成示例）",
      "en": "Configuration root (synthetic example)"
    },
    "root": "/example/claude-dev",
    "diffs": [
      {
        "key": "DISABLE_ERROR_REPORTING",
        "before": "—",
        "after": "\"1\"",
        "action": {
          "zh": "关闭",
          "en": "disable"
        }
      },
      {
        "key": "DISABLE_TELEMETRY",
        "before": "\"false\"",
        "after": "\"1\"",
        "action": {
          "zh": "关闭",
          "en": "disable"
        }
      },
      {
        "key": "DISABLE_FEEDBACK_COMMAND",
        "before": "\"1\"",
        "after": "\"1\"",
        "action": {
          "zh": "保留",
          "en": "keep"
        }
      }
    ],
    "note": {
      "zh": "合成字段示例。实际预览显示当前目标、读取值、版本条件与计划摘要。",
      "en": "Synthetic field example. A real preview shows the target, observed values, version conditions and plan digest."
    }
  },
  {
    "id": "settings-04",
    "group": "settings",
    "page": 4,
    "kind": "settings-effects",
    "eyebrow": {
      "zh": "互不替代的证据",
      "en": "DISTINCT EVIDENCE"
    },
    "title": {
      "zh": "配置读回与网络观察",
      "en": "Config readback and network observation"
    },
    "lead": {
      "zh": "配置是否写入、通道里看到了什么、出口测试结果，分别查看。",
      "en": "Check what was written, what the channel observed and what egress tests returned."
    },
    "outcomes": [
      {
        "title": {
          "zh": "配置读回",
          "en": "Config readback"
        },
        "text": {
          "zh": "字段已写入并再次读取。",
          "en": "The field was written and read again."
        },
        "limit": {
          "zh": "现有会话不会自动重载。",
          "en": "Existing sessions do not reload automatically."
        }
      },
      {
        "title": {
          "zh": "受控通道",
          "en": "Controlled channel"
        },
        "text": {
          "zh": "记录经过 Lintel 通道的请求。",
          "en": "Requests through the Lintel channel were observed."
        },
        "limit": {
          "zh": "不覆盖直接连接、UDP 或其他进程。",
          "en": "Does not cover direct sockets, UDP or other processes."
        }
      },
      {
        "title": {
          "zh": "网络与遥测测试",
          "en": "Network & telemetry tests"
        },
        "text": {
          "zh": "分别观察默认路径、通道出口和指定目标。",
          "en": "Observe default routes, channel egress and selected targets separately."
        },
        "limit": {
          "zh": "测试请求不证明全部 Claude 流量。",
          "en": "Test requests do not prove all Claude traffic."
        }
      }
    ],
    "note": {
      "zh": "配置读回和测试请求，不能证明全部 Claude 流量受到控制。",
      "en": "Config readback and test requests do not prove control over all Claude traffic."
    }
  },
  {
    "id": "settings-05",
    "group": "settings",
    "page": 5,
    "kind": "settings-restore",
    "eyebrow": {
      "zh": "原任务 · 当前值检查",
      "en": "ORIGINAL TASK · CURRENT-VALUE CHECK"
    },
    "title": {
      "zh": "按字段恢复",
      "en": "Restore selected fields"
    },
    "lead": {
      "zh": "从原任务选择要恢复的字段；遇到后续编辑和冲突，保留当前值。",
      "en": "Choose fields from the original task. Keep current values when later edits cause a conflict."
    },
    "graph": {
      "original": {
        "label": {
          "zh": "原任务写入",
          "en": "Written by original task"
        },
        "value": "DISABLE_TELEMETRY = \"1\""
      },
      "later": {
        "label": {
          "zh": "后来编辑",
          "en": "Later edit"
        },
        "value": "DISABLE_TELEMETRY = \"0\""
      },
      "conclusion": {
        "zh": "冲突：当前值已变化",
        "en": "Conflict: current value changed"
      },
      "result": {
        "zh": "保留当前值；无关字段不动",
        "en": "Keep the current value; unrelated fields stay intact"
      }
    },
    "note": {
      "zh": "示意流程。按字段恢复，不覆盖整份旧目录；不恢复凭据或浏览器数据。",
      "en": "Illustrative flow. Restore individual fields, not an old directory; credentials and browser data are excluded."
    },
    "qualifier": {
      "zh": "原任务仍可查询。不同的修改，需要新的预览与批准。",
      "en": "The original task stays queryable. Different changes require a new preview and approval."
    }
  }
];
