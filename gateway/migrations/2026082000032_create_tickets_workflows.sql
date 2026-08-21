-- ============================================================
-- 工单 + 工作流引擎 迁移 V2 (对齐 Rust 代码结构)
-- 先清理旧表再重建：DROP → CREATE
-- ============================================================

DROP TABLE IF EXISTS ticket_number_seq;
DROP TABLE IF EXISTS ticket_alert_links;
DROP TABLE IF EXISTS ticket_comments;
DROP TABLE IF EXISTS ticket_workflow_nodes;
DROP TABLE IF EXISTS tickets;
DROP TABLE IF EXISTS workflow_templates;

-- ----------------------------------------------------------
-- 工单主表 (rust: INSERT 里的所有列对应)
-- ----------------------------------------------------------
CREATE TABLE tickets (
  id               VARCHAR(36)  NOT NULL PRIMARY KEY,
  ticket_no        VARCHAR(64)  NOT NULL UNIQUE COMMENT 'WO-YYYYMMDD-NNN 可读编号',
  ticket_type      VARCHAR(32)  NOT NULL DEFAULT 'incident' COMMENT 'incident/problem/change/change_emergency/task',
  title            VARCHAR(512) NOT NULL,
  description      MEDIUMTEXT   NULL,
  priority         TINYINT      NOT NULL DEFAULT 3 COMMENT '1P1 2P2 3P3 4P4',
  category         VARCHAR(128) NULL,
  status           VARCHAR(32)  NOT NULL DEFAULT 'open' COMMENT 'open/assigned/in_progress/pending_review/resolved/closed/cancelled',
  reporter_id      VARCHAR(36)  NOT NULL,
  assignee_id      VARCHAR(36)  NULL,
  template_id      VARCHAR(36)  NULL,
  current_node_key VARCHAR(64)  NULL,
  sla_due_at       DATETIME     NULL,
  resolution       TEXT         NULL,
  closed_at        DATETIME     NULL,
  deleted_at       DATETIME     NULL,
  created_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  KEY idx_tickets_status        (status),
  KEY idx_tickets_priority      (priority),
  KEY idx_tickets_type          (ticket_type),
  KEY idx_tickets_category      (category),
  KEY idx_tickets_assignee      (assignee_id),
  KEY idx_tickets_reporter      (reporter_id),
  KEY idx_tickets_template      (template_id),
  KEY idx_tickets_sla           (sla_due_at),
  KEY idx_tickets_created       (created_at),
  KEY idx_tickets_deleted       (deleted_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单主表 (MeridianOps V2)';

-- ----------------------------------------------------------
-- 工单编号 (防并发碰撞)
-- ----------------------------------------------------------
CREATE TABLE ticket_number_seq (
  id          VARCHAR(36) NOT NULL PRIMARY KEY,
  date_prefix VARCHAR(16) NOT NULL,
  seq         INT         NOT NULL,
  created_at  DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uk_date_seq (date_prefix, seq)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单每日序号占位表';

-- ----------------------------------------------------------
-- 工单运行时节点
-- ----------------------------------------------------------
CREATE TABLE ticket_workflow_nodes (
  id               VARCHAR(36)  NOT NULL PRIMARY KEY,
  ticket_id        VARCHAR(36)  NOT NULL,
  node_key         VARCHAR(64)  NOT NULL,
  node_name        VARCHAR(128) NOT NULL,
  node_type        VARCHAR(32)  NOT NULL,
  node_index       INT          NOT NULL DEFAULT 0,
  approvers        JSON         NULL COMMENT '[{"id","name"}]',
  outs             JSON         NULL COMMENT '出边：[{to, condition, priority}]',
  status           VARCHAR(16)  NOT NULL DEFAULT 'pending' COMMENT 'pending/active/done/rejected/skipped',
  decision         VARCHAR(16)  NULL COMMENT 'approve/reject/skip',
  decider_id       VARCHAR(36)  NULL,
  entered_at       DATETIME     NULL,
  done_at          DATETIME     NULL,
  timeout_hours    INT          NULL,
  timeout_action   VARCHAR(32)  NULL,
  reject_back_to   VARCHAR(64)  NULL,
  extra            JSON         NULL,
  created_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uk_ticket_node (ticket_id, node_key),
  KEY idx_wn_status (status),
  KEY idx_wn_due    (done_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单运行时工作流节点';

-- ----------------------------------------------------------
-- 评论 / 审计轨迹
-- ----------------------------------------------------------
CREATE TABLE ticket_comments (
  id          VARCHAR(36)  NOT NULL PRIMARY KEY,
  ticket_id   VARCHAR(36)  NOT NULL,
  user_id     VARCHAR(36)  NULL,
  action      VARCHAR(32)  NOT NULL DEFAULT 'comment' COMMENT 'create/comment/assign/approve/reject/reassign/close/cancel/link_alert/unlink_alert',
  node_key    VARCHAR(64)  NULL,
  content     TEXT         NULL,
  extra       JSON         NULL,
  created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY idx_comment_ticket (ticket_id),
  KEY idx_comment_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单评论与流程审计';

-- ----------------------------------------------------------
-- 工单 - 告警关联
-- ----------------------------------------------------------
CREATE TABLE ticket_alert_links (
  id         VARCHAR(36)  NOT NULL PRIMARY KEY,
  ticket_id  VARCHAR(36)  NOT NULL,
  alert_id   VARCHAR(36)  NOT NULL,
  relation   VARCHAR(32)  NOT NULL DEFAULT 'caused_by',
  created_at DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uk_ticket_alert (ticket_id, alert_id),
  KEY idx_link_alert (alert_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单告警关联';

-- ----------------------------------------------------------
-- 工作流模板 (LogicFlow definition 存储)
-- ----------------------------------------------------------
CREATE TABLE workflow_templates (
  id            VARCHAR(36)  NOT NULL PRIMARY KEY,
  name          VARCHAR(128) NOT NULL,
  display_name  VARCHAR(128) NULL,
  ticket_type   VARCHAR(32)  NOT NULL COMMENT 'incident/problem/change/change_emergency/task',
  category      VARCHAR(128) NULL,
  definition    JSON         NOT NULL COMMENT '{nodes, edges} LogicFlow graph',
  version       INT          NOT NULL DEFAULT 1,
  enabled       TINYINT(1)   NOT NULL DEFAULT 1,
  scope         VARCHAR(16)  NOT NULL DEFAULT 'custom' COMMENT 'builtin/custom',
  description   VARCHAR(512) NULL,
  created_by    VARCHAR(36)  NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  deleted_at    DATETIME     NULL,
  KEY idx_wt_type    (ticket_type),
  KEY idx_wt_scope   (scope),
  KEY idx_wt_enabled (enabled),
  KEY idx_wt_del     (deleted_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='流程模板';

-- ============================================================
-- 种子权限 (ticket:*, workflow:*)
-- ============================================================
INSERT IGNORE INTO permissions (id, code, name, module, description) VALUES
(UUID_TO_BIN(UUID(),1), 'ticket:read',   '工单查看',   'ticket',   '查看工单列表、详情、KPI、评论'),
(UUID_TO_BIN(UUID(),1), 'ticket:create', '工单创建',   'ticket',   '创建工单并启动流程'),
(UUID_TO_BIN(UUID(),1), 'ticket:update', '工单更新',   'ticket',   '编辑工单、指派处理人、流程动作、关闭/取消'),
(UUID_TO_BIN(UUID(),1), 'ticket:delete', '工单删除',   'ticket',   '软删除工单'),
(UUID_TO_BIN(UUID(),1), 'workflow:read',  '流程模板查看', 'workflow', '查看模板列表、详情、预览编译'),
(UUID_TO_BIN(UUID(),1), 'workflow:admin', '流程模板管理', 'workflow', '创建/编辑/启停/删除流程模板');

-- admin & operator 角色都给工单权限；viewer 只给只读权限
SET @admin_id = (SELECT id FROM roles WHERE name='admin' LIMIT 1);
SET @op_id    = (SELECT id FROM roles WHERE name='operator' LIMIT 1);
SET @viewer_id= (SELECT id FROM roles WHERE name='viewer' LIMIT 1);

INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT @admin_id, id FROM permissions WHERE code IN ('ticket:read','ticket:create','ticket:update','ticket:delete','workflow:read','workflow:admin') AND @admin_id IS NOT NULL;

INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT @op_id, id FROM permissions WHERE code IN ('ticket:read','ticket:create','ticket:update','workflow:read') AND @op_id IS NOT NULL;

INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT @viewer_id, id FROM permissions WHERE code IN ('ticket:read','workflow:read') AND @viewer_id IS NOT NULL;

-- ============================================================
-- 5 个种子模板（Definition 是 LogicFlow 结构）
-- 每个 template id 用固定 UUID 保证幂等
-- ============================================================

-- T1: 事件标准流程 incident_standard
--   start -> dispatch(分派 auto_pass) -> l1_triage(单人审批) -> l2_investigate(会签 P3/P4) -> resolve(单人审批 P1/P2) -> verify(单人审批) -> end
INSERT IGNORE INTO workflow_templates (id, name, display_name, ticket_type, category, definition, version, enabled, scope, description, created_by, created_at)
VALUES(
  '00000000-0000-0000-0000-000000000001',
  'incident_standard',
  '事件工单标准流程',
  'incident',
  '标准',
  JSON_OBJECT(
    'nodes', JSON_ARRAY(
      JSON_OBJECT('id','n1','type','start','x', 40,'y',220,'properties',JSON_OBJECT()),
      JSON_OBJECT('id','n2','type','auto_pass','x',180,'y',220,
                  'properties',JSON_OBJECT('key','dispatch','name','事件分派','approverSelector',JSON_ARRAY('task_dispatcher'),'rejectBackTo','dispatch',
                                           'outs',JSON_ARRAY())),
      JSON_OBJECT('id','n3','type','single_approval','x',340,'y',220,
                  'properties',JSON_OBJECT('key','l1_triage','name','一线确认/分类','approverSelector',JSON_ARRAY('role:operator'),'timeoutHours',2,'timeoutAction','escalate','rejectBackTo','dispatch')),
      JSON_OBJECT('id','n4','type','countersign','x',520,'y',120,
                  'properties',JSON_OBJECT('key','l2_investigate','name','二线排查(会签)','approverSelector',JSON_ARRAY('senior_engineer_group'),'timeoutHours',24,'timeoutAction','escalate','rejectBackTo','l1_triage')),
      JSON_OBJECT('id','n5','type','single_approval','x',520,'y',320,
                  'properties',JSON_OBJECT('key','resolve','name','处理解决审批','approverSelector',JSON_ARRAY('incident_manager'),'timeoutHours',8,'timeoutAction','escalate','rejectBackTo','l2_investigate')),
      JSON_OBJECT('id','n6','type','condition_gateway','x',720,'y',220,
                  'properties',JSON_OBJECT('key','cg_check','name','P1/P2 复核分支','approverSelector',NULL,'rejectBackTo',NULL)),
      JSON_OBJECT('id','n7','type','all_approval','x',900,'y',120,
                  'properties',JSON_OBJECT('key','mgr_review','name','经理复核','approverSelector',JSON_ARRAY('department_head_of_reporter'),'timeoutHours',4,'timeoutAction','auto_pass','rejectBackTo','l2_investigate')),
      JSON_OBJECT('id','n8','type','single_approval','x',900,'y',320,
                  'properties',JSON_OBJECT('key','verify','name','提报人验证','approverSelector',JSON_ARRAY('tester'),'timeoutHours',48,'timeoutAction','auto_close','rejectBackTo','l2_investigate')),
      JSON_OBJECT('id','n9','type','end','x',1100,'y',220,'properties',JSON_OBJECT())
    ),
    'edges', JSON_ARRAY(
      JSON_OBJECT('id','e1','sourceNodeId','n1','targetNodeId','n2','properties',JSON_OBJECT()),
      JSON_OBJECT('id','e2','sourceNodeId','n2','targetNodeId','n3','properties',JSON_OBJECT()),
      JSON_OBJECT('id','e3','sourceNodeId','n3','targetNodeId','n4','properties',JSON_OBJECT('condition',JSON_OBJECT('field','priority','op','>=','value',3),'priority',2)),
      JSON_OBJECT('id','e4','sourceNodeId','n3','targetNodeId','n5','properties',JSON_OBJECT('condition',JSON_OBJECT('field','priority','op','<=','value',2),'priority',1)),
      JSON_OBJECT('id','e5','sourceNodeId','n4','targetNodeId','n6','properties',JSON_OBJECT()),
      JSON_OBJECT('id','e6','sourceNodeId','n5','targetNodeId','n6','properties',JSON_OBJECT()),
      JSON_OBJECT('id','e7','sourceNodeId','n6','targetNodeId','n7','properties',JSON_OBJECT('condition',JSON_OBJECT('field','priority','op','<=','value',2),'priority',1)),
      JSON_OBJECT('id','e8','sourceNodeId','n6','targetNodeId','n8','properties',JSON_OBJECT('condition',JSON_OBJECT('field','priority','op','>=','value',3),'priority',2)),
      JSON_OBJECT('id','e9','sourceNodeId','n7','targetNodeId','n9','properties',JSON_OBJECT()),
      JSON_OBJECT('id','e10','sourceNodeId','n8','targetNodeId','n9','properties',JSON_OBJECT())
    )
  ),
  1, 1, 'builtin', '标准事件处理：分派→一线→二线/解决→条件分支→复核/验证→结束', NULL, NOW()
);

-- T2: problem_standard
INSERT IGNORE INTO workflow_templates (id, name, display_name, ticket_type, category, definition, version, enabled, scope, description, created_by, created_at)
VALUES(
  '00000000-0000-0000-0000-000000000002',
  'problem_standard',
  '故障标准流程',
  'problem',
  '标准',
  JSON_OBJECT(
    'nodes', JSON_ARRAY(
      JSON_OBJECT('id','p1','type','start','x',40,'y',240,'properties',JSON_OBJECT()),
      JSON_OBJECT('id','p2','type','auto_pass','x',180,'y',240,
                  'properties',JSON_OBJECT('key','dispatch','name','故障受理分派')),
      JSON_OBJECT('id','p3','type','single_approval','x',340,'y',240,
                  'properties',JSON_OBJECT('key','rca_plan','name','根因方案审批','approverSelector',JSON_ARRAY('problem_manager'),'rejectBackTo','dispatch')),
      JSON_OBJECT('id','p4','type','countersign','x',520,'y',240,
                  'properties',JSON_OBJECT('key','implement_fix','name','实施修复(会签)','approverSelector',JSON_ARRAY('senior_engineer_group'),'rejectBackTo','rca_plan')),
      JSON_OBJECT('id','p5','type','single_approval','x',720,'y',240,
                  'properties',JSON_OBJECT('key','verify','name','修复验证','approverSelector',JSON_ARRAY('tester'),'rejectBackTo','implement_fix')),
      JSON_OBJECT('id','p6','type','single_approval','x',900,'y',240,
                  'properties',JSON_OBJECT('key','closure','name','关闭确认','approverSelector',JSON_ARRAY('problem_manager'))),
      JSON_OBJECT('id','p7','type','end','x',1060,'y',240,'properties',JSON_OBJECT())
    ),
    'edges', JSON_ARRAY(
      JSON_OBJECT('id','pe1','sourceNodeId','p1','targetNodeId','p2','properties',JSON_OBJECT()),
      JSON_OBJECT('id','pe2','sourceNodeId','p2','targetNodeId','p3','properties',JSON_OBJECT()),
      JSON_OBJECT('id','pe3','sourceNodeId','p3','targetNodeId','p4','properties',JSON_OBJECT()),
      JSON_OBJECT('id','pe4','sourceNodeId','p4','targetNodeId','p5','properties',JSON_OBJECT()),
      JSON_OBJECT('id','pe5','sourceNodeId','p5','targetNodeId','p6','properties',JSON_OBJECT()),
      JSON_OBJECT('id','pe6','sourceNodeId','p6','targetNodeId','p7','properties',JSON_OBJECT())
    )
  ),
  1, 1, 'builtin', '故障工单：受理 → 根因方案 → 实施修复 → 验证 → 关闭确认', NULL, NOW()
);

-- T3: change_normal
INSERT IGNORE INTO workflow_templates (id, name, display_name, ticket_type, category, definition, version, enabled, scope, description, created_by, created_at)
VALUES(
  '00000000-0000-0000-0000-000000000003',
  'change_normal',
  '标准变更流程',
  'change',
  '标准',
  JSON_OBJECT(
    'nodes', JSON_ARRAY(
      JSON_OBJECT('id','c1','type','start','x',40,'y',220,'properties',JSON_OBJECT()),
      JSON_OBJECT('id','c2','type','auto_pass','x',180,'y',220,'properties',JSON_OBJECT('key','draft','name','创建/提交')),
      JSON_OBJECT('id','c3','type','single_approval','x',340,'y',220,
                  'properties',JSON_OBJECT('key','dept_approve','name','部门主管审批','approverSelector',JSON_ARRAY('department_head_of_reporter'),'rejectBackTo','draft')),
      JSON_OBJECT('id','c4','type','all_approval','x',520,'y',220,
                  'properties',JSON_OBJECT('key','cab_review','name','CAB 评审','approverSelector',JSON_ARRAY('cab_member'),'rejectBackTo','dept_approve')),
      JSON_OBJECT('id','c5','type','single_approval','x',720,'y',220,
                  'properties',JSON_OBJECT('key','implement','name','变更实施','approverSelector',JSON_ARRAY('assignee'),'rejectBackTo','cab_review')),
      JSON_OBJECT('id','c6','type','single_approval','x',900,'y',220,
                  'properties',JSON_OBJECT('key','verify_close','name','验证关闭','approverSelector',JSON_ARRAY('tester'))),
      JSON_OBJECT('id','c7','type','end','x',1060,'y',220,'properties',JSON_OBJECT())
    ),
    'edges', JSON_ARRAY(
      JSON_OBJECT('id','ce1','sourceNodeId','c1','targetNodeId','c2','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ce2','sourceNodeId','c2','targetNodeId','c3','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ce3','sourceNodeId','c3','targetNodeId','c4','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ce4','sourceNodeId','c4','targetNodeId','c5','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ce5','sourceNodeId','c5','targetNodeId','c6','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ce6','sourceNodeId','c6','targetNodeId','c7','properties',JSON_OBJECT())
    )
  ),
  1, 1, 'builtin', '标准变更：提交 → 部门审批 → CAB 评审 → 实施 → 验证关闭', NULL, NOW()
);

-- T4: change_emergency
INSERT IGNORE INTO workflow_templates (id, name, display_name, ticket_type, category, definition, version, enabled, scope, description, created_by, created_at)
VALUES(
  '00000000-0000-0000-0000-000000000004',
  'change_emergency',
  '紧急变更流程',
  'change_emergency',
  '紧急',
  JSON_OBJECT(
    'nodes', JSON_ARRAY(
      JSON_OBJECT('id','g1','type','start','x',40,'y',220,'properties',JSON_OBJECT()),
      JSON_OBJECT('id','g2','type','auto_pass','x',180,'y',220,'properties',JSON_OBJECT('key','create','name','紧急提单')),
      JSON_OBJECT('id','g3','type','any_approval','x',360,'y',220,
                  'properties',JSON_OBJECT('key','mgr_approve','name','主管/Vp审批','approverSelector',JSON_ARRAY('vp_oncall','department_head_of_reporter'),'rejectBackTo','create')),
      JSON_OBJECT('id','g4','type','single_approval','x',560,'y',220,
                  'properties',JSON_OBJECT('key','implement','name','紧急实施','approverSelector',JSON_ARRAY('assignee'),'rejectBackTo','mgr_approve')),
      JSON_OBJECT('id','g5','type','single_approval','x',760,'y',220,
                  'properties',JSON_OBJECT('key','verify','name','事后验证','approverSelector',JSON_ARRAY('incident_manager'))),
      JSON_OBJECT('id','g6','type','end','x',920,'y',220,'properties',JSON_OBJECT())
    ),
    'edges', JSON_ARRAY(
      JSON_OBJECT('id','ge1','sourceNodeId','g1','targetNodeId','g2','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ge2','sourceNodeId','g2','targetNodeId','g3','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ge3','sourceNodeId','g3','targetNodeId','g4','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ge4','sourceNodeId','g4','targetNodeId','g5','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ge5','sourceNodeId','g5','targetNodeId','g6','properties',JSON_OBJECT())
    )
  ),
  1, 1, 'builtin', '紧急变更：主管/Vp 任一审批 → 实施 → 验证(事后CAB补审)', NULL, NOW()
);

-- T5: task_simple
INSERT IGNORE INTO workflow_templates (id, name, display_name, ticket_type, category, definition, version, enabled, scope, description, created_by, created_at)
VALUES(
  '00000000-0000-0000-0000-000000000005',
  'task_simple',
  '运维任务简单流程',
  'task',
  '日常',
  JSON_OBJECT(
    'nodes', JSON_ARRAY(
      JSON_OBJECT('id','k1','type','start','x',40,'y',220,'properties',JSON_OBJECT()),
      JSON_OBJECT('id','k2','type','auto_pass','x',180,'y',220,'properties',JSON_OBJECT('key','dispatch','name','任务分派')),
      JSON_OBJECT('id','k3','type','single_approval','x',360,'y',220,
                  'properties',JSON_OBJECT('key','do','name','执行执行','approverSelector',JSON_ARRAY('assignee'),'rejectBackTo','dispatch')),
      JSON_OBJECT('id','k4','type','single_approval','x',560,'y',220,
                  'properties',JSON_OBJECT('key','accept','name','验收','approverSelector',JSON_ARRAY('reporter'))),
      JSON_OBJECT('id','k5','type','end','x',720,'y',220,'properties',JSON_OBJECT())
    ),
    'edges', JSON_ARRAY(
      JSON_OBJECT('id','ke1','sourceNodeId','k1','targetNodeId','k2','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ke2','sourceNodeId','k2','targetNodeId','k3','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ke3','sourceNodeId','k3','targetNodeId','k4','properties',JSON_OBJECT()),
      JSON_OBJECT('id','ke4','sourceNodeId','k4','targetNodeId','k5','properties',JSON_OBJECT())
    )
  ),
  1, 1, 'builtin', '日常运维任务：分派 → 执行 → 提报人验收 → 结束', NULL, NOW()
);