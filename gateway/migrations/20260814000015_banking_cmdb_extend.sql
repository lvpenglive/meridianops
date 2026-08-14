-- ============================================================
-- 银行运维体系扩展：新增 CI 模型 + 关系类型 + 补充属性
-- ============================================================

-- ============================================================
-- 1. 新增 CI 模型（7 个）
-- ============================================================

INSERT INTO ci_models (id, code, name, icon, description, enabled, sort_order, created_at, updated_at) VALUES
('cmdb-model-security',    'security_device', '安全设备', 'Lock',      '堡垒机/WAF/IDS/IPS/加密机/动态令牌',     1, 6,  NOW(), NOW()),
('cmdb-model-storage',     'storage_device',  '存储设备', 'Box',       'SAN/NAS/存储阵列，银行核心系统专用存储',   1, 7,  NOW(), NOW()),
('cmdb-model-cluster',     'cluster',         '集群',     'Grid',      'WebLogic集群/Redis集群/MySQL主从等实例组', 1, 8,  NOW(), NOW()),
('cmdb-model-lb',          'load_balancer',   '负载均衡', 'ScaleToOriginal', 'F5/Nginx Plus/HAProxy，VIP与后端池管理', 1, 9,  NOW(), NOW()),
('cmdb-model-vip',         'virtual_ip',      '虚拟IP/域名', 'Link',   'VIP/浮动IP/域名，双活与灾备关键节点',     1, 10, NOW(), NOW()),
('cmdb-model-datacenter',  'datacenter',      '机房/数据中心', 'OfficeBuilding', '物理机房/可用区，异地灾备与双活',  1, 11, NOW(), NOW()),
('cmdb-model-batch',       'batch_job',       '批处理作业', 'Timer',   '日终结算/对账/报表等银行批处理任务',     1, 12, NOW(), NOW())
ON DUPLICATE KEY UPDATE id = id;

-- ============================================================
-- 2. 新增模型属性
-- ============================================================

-- 安全设备属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-sec-type',    'cmdb-model-security', 'device_type', '设备类型', 'enum', '', '["堡垒机","WAF","IDS","IPS","加密机","动态令牌","VPN","Other"]', 1, 0, 1, 1, NOW()),
('attr-sec-name',    'cmdb-model-security', 'device_name', '设备名',   'string', '', NULL, 1, 1, 1, 2, NOW()),
('attr-sec-ip',      'cmdb-model-security', 'mgmt_ip',     '管理IP',   'string', '', NULL, 1, 0, 1, 3, NOW()),
('attr-sec-vendor',  'cmdb-model-security', 'vendor',      '厂商',     'enum',   '', '["帕拉迪","深信服","绿盟","启明星辰","天融信","Other"]', 0, 0, 1, 4, NOW()),
('attr-sec-model',   'cmdb-model-security', 'device_model','设备型号', 'string', '', NULL, 0, 0, 0, 5, NOW()),
('attr-sec-status',  'cmdb-model-security', 'deploy_mode', '部署方式', 'enum',   '串联', '["串联","旁路","离线"]', 0, 0, 0, 6, NOW());

-- 存储设备属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-stor-type',    'cmdb-model-storage', 'storage_type', '存储类型', 'enum',   '', '["SAN","NAS","存储阵列","分布式存储","备份存储"]', 1, 0, 1, 1, NOW()),
('attr-stor-name',    'cmdb-model-storage', 'device_name',  '设备名',   'string', '', NULL, 1, 1, 1, 2, NOW()),
('attr-stor-vendor',  'cmdb-model-storage', 'vendor',       '厂商',     'enum',   '', '["EMC","HDS","华为","NetApp","Pure Storage","Other"]', 0, 0, 1, 3, NOW()),
('attr-stor-cap',     'cmdb-model-storage', 'capacity_tb',  '容量(TB)', 'number', '10', NULL, 0, 0, 1, 4, NOW()),
('attr-stor-raid',    'cmdb-model-storage', 'raid_level',   'RAID级别', 'string', 'RAID5', NULL, 0, 0, 0, 5, NOW()),
('attr-stor-protocol','cmdb-model-storage', 'protocol',    '协议',     'enum',   '', '["FC","iSCSI","NFS","SMB","S3"]', 0, 0, 0, 6, NOW());

-- 集群属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-cl-type',    'cmdb-model-cluster', 'cluster_type', '集群类型', 'enum',   '', '["WebLogic集群","Redis集群","MySQL主从","Oracle RAC","Kafka集群","ES集群","Other"]', 1, 0, 1, 1, NOW()),
('attr-cl-name',    'cmdb-model-cluster', 'cluster_name', '集群名',   'string', '', NULL, 1, 1, 1, 2, NOW()),
('attr-cl-nodes',   'cmdb-model-cluster', 'node_count',   '节点数',   'number', '2', NULL, 0, 0, 1, 3, NOW()),
('attr-cl-mode',    'cmdb-model-cluster', 'ha_mode',      '高可用模式', 'enum', '主备', '["主备","双活","多活","单点"]', 0, 0, 1, 4, NOW());

-- 负载均衡属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-lb-type',    'cmdb-model-lb', 'lb_type',     '类型',       'enum',   '', '["F5","Nginx Plus","HAProxy","LVS","AWS ALB","Other"]', 1, 0, 1, 1, NOW()),
('attr-lb-vip',     'cmdb-model-lb', 'vip',         '虚拟IP',     'string', '', NULL, 1, 0, 1, 2, NOW()),
('attr-lb-name',    'cmdb-model-lb', 'device_name', '设备名',     'string', '', NULL, 1, 1, 1, 3, NOW()),
('attr-lb-pool',    'cmdb-model-lb', 'pool_members','后端成员数', 'number', '2', NULL, 0, 0, 0, 4, NOW()),
('attr-lb-hc',      'cmdb-model-lb', 'health_check','健康检查',   'enum',   'TCP', '["TCP","HTTP","HTTPS","ICMP"]', 0, 0, 0, 5, NOW()),
('attr-lb-mode',    'cmdb-model-lb', 'lb_mode',     '调度算法',   'enum',   'round_robin', '["round_robin","least_connections","source_hash","weighted"]', 0, 0, 0, 6, NOW());

-- 虚拟IP/域名属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-vip-addr',   'cmdb-model-vip', 'vip_address', 'VIP/域名',  'string', '', NULL, 1, 1, 1, 1, NOW()),
('attr-vip-type',   'cmdb-model-vip', 'vip_type',    '类型',      'enum',   '', '["VIP","浮动IP","域名","VIP+域名"]', 1, 0, 1, 2, NOW()),
('attr-vip-role',   'cmdb-model-vip', 'role',        '角色',      'enum',   '主', '["主","备","双活"]', 0, 0, 1, 3, NOW()),
('attr-vip-port',   'cmdb-model-vip', 'port',        '端口',      'number', '80', NULL, 0, 0, 0, 4, NOW());

-- 机房/数据中心属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-dc-name',    'cmdb-model-datacenter', 'dc_name',     '数据中心名称', 'string', '', NULL, 1, 1, 1, 1, NOW()),
('attr-dc-loc',     'cmdb-model-datacenter', 'location',    '地理位置',     'string', '', NULL, 1, 0, 1, 2, NOW()),
('attr-dc-type',    'cmdb-model-datacenter', 'dc_type',     '中心类型',     'enum',   '', '["主中心","同城灾备","异地灾备","双活中心"]', 1, 0, 1, 3, NOW()),
('attr-dc-tier',    'cmdb-model-datacenter', 'tier_level',  'Tier等级',     'enum',   'III', '["I","II","III","IV"]', 0, 0, 1, 4, NOW()),
('attr-dc-racks',   'cmdb-model-datacenter', 'rack_count',  '机柜数',       'number', '50', NULL, 0, 0, 0, 5, NOW());

-- 批处理作业属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-batch-name',   'cmdb-model-batch', 'job_name',     '作业名称',   'string', '', NULL, 1, 1, 1, 1, NOW()),
('attr-batch-cat',    'cmdb-model-batch', 'category',     '作业类别',   'enum',   '', '["日终结算","对账","报表","数据清理","批处理调度"]', 1, 0, 1, 2, NOW()),
('attr-batch-sched',  'cmdb-model-batch', 'schedule',     '调度周期',   'string', '每日 00:00', NULL, 0, 0, 1, 3, NOW()),
('attr-batch-dur',    'cmdb-model-batch', 'avg_duration', '平均耗时(分钟)', 'number', '30', NULL, 0, 0, 0, 4, NOW()),
('attr-batch-status', 'cmdb-model-batch', 'job_status',   '状态',       'enum',   '启用', '["启用","停用","异常"]', 0, 0, 1, 5, NOW());

-- ============================================================
-- 3. 补充现有模型缺失属性
-- ============================================================

-- 业务系统补充属性：负责人、等保级别、灾备级别
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-bs-owner',   'cmdb-model-business', 'owner',       '负责人',   'string', '', NULL, 0, 0, 1, 5, NOW()),
('attr-bs-dept',    'cmdb-model-business', 'department',  '所属部门', 'string', '', NULL, 0, 0, 1, 6, NOW()),
('attr-bs-sec',     'cmdb-model-business', 'sec_level',   '等保级别', 'enum',   '3', '["2","3","4"]', 0, 0, 1, 7, NOW()),
('attr-bs-dr',      'cmdb-model-business', 'dr_level',    '灾备级别', 'enum',   '同城灾备', '["无灾备","同城灾备","异地灾备","双活"]', 0, 0, 1, 8, NOW())
ON DUPLICATE KEY UPDATE id = id;

-- 主机补充属性：虚拟化类型、机柜位置
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-host-vmtype','cmdb-model-host', 'vm_type',   '虚拟化类型', 'enum',   '物理机', '["物理机","VMware","KVM","Xen","其他"]', 0, 0, 1, 8, NOW()),
('attr-host-rack',  'cmdb-model-host', 'rack_pos',  '机柜位置',   'string', '', NULL, 0, 0, 0, 9, NOW())
ON DUPLICATE KEY UPDATE id = id;

-- 网络设备补充属性：设备类型
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-nd-type',    'cmdb-model-network', 'device_type', '设备类型', 'enum', '', '["交换机","路由器","防火墙","负载均衡","VPN网关","其他"]', 1, 0, 1, 0, NOW())
ON DUPLICATE KEY UPDATE id = id;

-- ============================================================
-- 4. 新增关系类型（3 个）
-- ============================================================

INSERT INTO ci_relation_types (id, code, name, description, directional, enabled, sort_order, created_at, updated_at) VALUES
    ('reltype-backs-up',  'backs_up',  '备份', 'A 是 B 的备份实例（如备库→主库）',           1, 1, 6, NOW(3), NOW(3)),
    ('reltype-syncs-to',  'syncs_to',  '同步', 'A 同步数据到 B（如 MySQL 主→从、OGG 同步）', 1, 1, 7, NOW(3), NOW(3)),
    ('reltype-monitors',  'monitors',  '监控', 'A 监控 B 的运行状态（如 Zabbix→主机）',       1, 1, 8, NOW(3), NOW(3))
ON DUPLICATE KEY UPDATE id = id;
