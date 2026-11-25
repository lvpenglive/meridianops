-- 主机双 IP：业务 IP（ip）+ 运维 IP（mgmt_ip）
-- 关联资源时两者都参与匹配；SSH/安装优先用运维 IP，空则回退业务 IP

-- 原「IP地址」统一改叫业务 IP（仅主机模型）
UPDATE ci_model_attrs
SET name = '业务IP'
WHERE model_id = 'cmdb-model-host' AND code = 'ip';

INSERT IGNORE INTO ci_model_attrs (
    id, model_id, code, name, value_type, default_value, options,
    is_required, is_unique, is_searchable, sort_order, created_at
) VALUES (
    'attr-host-mgmt-ip',
    'cmdb-model-host',
    'mgmt_ip',
    '运维IP',
    'string',
    '',
    NULL,
    0,
    0,
    1,
    3,
    UTC_TIMESTAMP()
);

-- 若环境里已有同 code 但旧名称，纠正文案
UPDATE ci_model_attrs
SET name = '运维IP'
WHERE model_id = 'cmdb-model-host' AND code = 'mgmt_ip';

-- 给原有主机属性腾一下顺序（从大到小改，避免中间态撞号）
UPDATE ci_model_attrs SET sort_order = 8 WHERE id = 'attr-host-room' AND sort_order = 7;
UPDATE ci_model_attrs SET sort_order = 7 WHERE id = 'attr-host-disk' AND sort_order = 6;
UPDATE ci_model_attrs SET sort_order = 6 WHERE id = 'attr-host-mem' AND sort_order = 5;
UPDATE ci_model_attrs SET sort_order = 5 WHERE id = 'attr-host-cpu' AND sort_order = 4;
UPDATE ci_model_attrs SET sort_order = 4 WHERE id = 'attr-host-os' AND sort_order = 3;
