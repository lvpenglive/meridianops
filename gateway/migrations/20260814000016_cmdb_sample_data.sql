-- ============================================================
-- 银行业 CMDB 样例数据
-- 典型三层架构：业务系统 → VIP/负载均衡 → 应用集群 → 中间件/数据库 → 主机 → 存储/网络
-- ============================================================

-- ---- 1. 机房/数据中心（2 个：主中心 + 同城灾备）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-dc01', 'cmdb-model-datacenter', '上海张江主中心',  'running', '{"dc_name":"SH-ZJ-MAIN","location":"上海浦东张江","dc_type":"主中心","tier_level":"IV","rack_count":120}', '核心机房',  NOW(), NOW()),
('ins-dc02', 'cmdb-model-datacenter', '上海宝山同城灾备','running', '{"dc_name":"SH-BS-DR","location":"上海宝山","dc_type":"同城灾备","tier_level":"III","rack_count":80}',  '灾备机房',  NOW(), NOW());

-- ---- 2. 网络设备（核心交换机、各机房边界防火墙、F5 负载均衡硬设备）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-sw-core1',  'cmdb-model-network', '核心交换机-SW01',  'running', '{"device_type":"交换机","device_name":"CORE-SW-01","mgmt_ip":"10.100.0.1","vendor":"Huawei","device_model":"CE12800","port_count":128}',  '核心网',  NOW(), NOW()),
('ins-sw-core2',  'cmdb-model-network', '核心交换机-SW02',  'running', '{"device_type":"交换机","device_name":"CORE-SW-02","mgmt_ip":"10.100.0.2","vendor":"Huawei","device_model":"CE12800","port_count":128}',  '核心网',  NOW(), NOW()),
('ins-fw-zj',     'cmdb-model-network', '张江边界防火墙',    'running', '{"device_type":"防火墙","device_name":"FW-ZJ-BORDER","mgmt_ip":"10.100.0.253","vendor":"Huawei","device_model":"USG9520","port_count":24}', '等保三级',NOW(), NOW()),
('ins-fw-bs',     'cmdb-model-network', '宝山边界防火墙',    'running', '{"device_type":"防火墙","device_name":"FW-BS-BORDER","mgmt_ip":"10.200.0.253","vendor":"Huawei","device_model":"USG9520","port_count":24}', '等保三级',NOW(), NOW()),
('ins-f5-app',    'cmdb-model-load_balancer','F5-应用交付',   'running', '{"lb_type":"F5","vip":"10.0.1.1","device_name":"F5-BIGIP-01","pool_members":8,"health_check":"HTTPS","lb_mode":"least_connections"}','7层负载',NOW(), NOW());

-- ---- 3. 安全设备（堡垒机、WAF、加密机）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-bastion',   'cmdb-model-security', '帕拉迪堡垒机',       'running', '{"device_type":"堡垒机","device_name":"Bastion-PLD-01","mgmt_ip":"10.100.1.10","vendor":"帕拉迪","device_model":"PAM-3000","deploy_mode":"串联"}','运维审计',NOW(), NOW()),
('ins-waf',       'cmdb-model-security', '深信服WAF',         'running', '{"device_type":"WAF","device_name":"WAF-SF-01","mgmt_ip":"10.100.1.11","vendor":"深信服","device_model":"WAF-10000","deploy_mode":"串联"}','Web防护', NOW(), NOW()),
('ins-hsm',       'cmdb-model-security', '江南天安加密机',    'running', '{"device_type":"加密机","device_name":"HSM-JNTA-01","mgmt_ip":"10.100.1.12","vendor":"Other","device_model":"SJJ1310","deploy_mode":"串联"}','密码机',  NOW(), NOW());

-- ---- 4. 存储设备（2 套 SAN）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-san-01',   'cmdb-model-storage', '核心存储-HDS-VSP',     'running', '{"storage_type":"SAN","device_name":"HDS-VSP-G1000-01","vendor":"HDS","capacity_tb":800,"raid_level":"RAID1+0","protocol":"FC"}','核心存储',NOW(), NOW()),
('ins-san-02',   'cmdb-model-storage', '备份存储-华为5500V5',  'running', '{"storage_type":"备份存储","device_name":"HW-OceanStor-5500V5-01","vendor":"华为","capacity_tb":400,"raid_level":"RAID5","protocol":"iSCSI"}','备份存储',NOW(), NOW());

-- ---- 5. 虚拟IP/域名（各业务系统对外入口）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-vip-core',  'cmdb-model-vip', '核心系统-VIP',       'running', '{"vip_address":"10.0.0.10","vip_type":"VIP","role":"主","port":8080}', '核心系统入口', NOW(), NOW()),
('ins-vip-ebank', 'cmdb-model-vip', '网银-VIP',          'running', '{"vip_address":"10.0.0.11","vip_type":"VIP+域名","role":"主","port":443}', '网银对外入口', NOW(), NOW()),
('ins-vip-mbank', 'cmdb-model-vip', '手机银行-域名',     'running', '{"vip_address":"m.bank.example.cn","vip_type":"域名","role":"双活","port":443}','手机银行入口',NOW(), NOW()),
('ins-vip-ecs',   'cmdb-model-vip', '企业服务总线-VIP',  'running', '{"vip_address":"10.0.0.20","vip_type":"VIP","role":"主","port":9080}', 'ESB入口',   NOW(), NOW());

-- ---- 6. 主机（虚拟机 + 物理机，共 18 台）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
-- 核心应用主机（WebLogic 集群 4 节点）
('ins-host-core-app1','cmdb-model-host','核心应用-APP01','running','{"hostname":"core-app-01","ip":"10.0.1.101","os":"RHEL 7","cpu":16,"memory":64,"disk":500,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R01-A1-01"}','核心系统,APP',NOW(),NOW()),
('ins-host-core-app2','cmdb-model-host','核心应用-APP02','running','{"hostname":"core-app-02","ip":"10.0.1.102","os":"RHEL 7","cpu":16,"memory":64,"disk":500,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R01-A1-02"}','核心系统,APP',NOW(),NOW()),
('ins-host-core-app3','cmdb-model-host','核心应用-APP03','running','{"hostname":"core-app-03","ip":"10.0.2.101","os":"RHEL 7","cpu":16,"memory":64,"disk":500,"datacenter":"SH-BS-DR","vm_type":"VMware","rack_pos":"R03-B1-01"}','灾备,核心系统',NOW(),NOW()),
('ins-host-core-app4','cmdb-model-host','核心应用-APP04','stopped','{"hostname":"core-app-04","ip":"10.0.2.102","os":"RHEL 7","cpu":16,"memory":64,"disk":500,"datacenter":"SH-BS-DR","vm_type":"VMware","rack_pos":"R03-B1-02"}','灾备,核心系统',NOW(),NOW()),
-- 核心数据库（Oracle RAC 两节点）
('ins-host-core-db1','cmdb-model-host','核心DB-NODE01','running','{"hostname":"core-db-01","ip":"10.0.1.201","os":"AIX","cpu":32,"memory":128,"disk":2000,"datacenter":"SH-ZJ-MAIN","vm_type":"物理机","rack_pos":"R02-C1-01"}','核心系统,DB',NOW(),NOW()),
('ins-host-core-db2','cmdb-model-host','核心DB-NODE02','running','{"hostname":"core-db-02","ip":"10.0.1.202","os":"AIX","cpu":32,"memory":128,"disk":2000,"datacenter":"SH-ZJ-MAIN","vm_type":"物理机","rack_pos":"R02-C1-02"}','核心系统,DB',NOW(),NOW()),
-- 网银应用主机（Nginx + WebLogic 4 台）
('ins-host-ebank-web1','cmdb-model-host','网银WEB-01','running','{"hostname":"ebank-web-01","ip":"10.0.1.31","os":"CentOS 7","cpu":8,"memory":16,"disk":200,"datacenter":"SH-ZJ-MAIN","vm_type":"KVM","rack_pos":"R01-B1-01"}','网银,WEB',NOW(),NOW()),
('ins-host-ebank-web2','cmdb-model-host','网银WEB-02','running','{"hostname":"ebank-web-02","ip":"10.0.1.32","os":"CentOS 7","cpu":8,"memory":16,"disk":200,"datacenter":"SH-ZJ-MAIN","vm_type":"KVM","rack_pos":"R01-B1-02"}','网银,WEB',NOW(),NOW()),
('ins-host-ebank-app1','cmdb-model-host','网银APP-01','running','{"hostname":"ebank-app-01","ip":"10.0.1.41","os":"RHEL 8","cpu":16,"memory":32,"disk":400,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R01-B2-01"}','网银,APP',NOW(),NOW()),
('ins-host-ebank-app2','cmdb-model-host','网银APP-02','running','{"hostname":"ebank-app-02","ip":"10.0.1.42","os":"RHEL 8","cpu":16,"memory":32,"disk":400,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R01-B2-02"}','网银,APP',NOW(),NOW()),
-- 手机银行应用
('ins-host-mbank-app1','cmdb-model-host','手机银行APP-01','running','{"hostname":"mbank-app-01","ip":"10.0.1.51","os":"RHEL 8","cpu":16,"memory":32,"disk":400,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R01-C2-01"}','手机银行,APP',NOW(),NOW()),
('ins-host-mbank-app2','cmdb-model-host','手机银行APP-02','running','{"hostname":"mbank-app-02","ip":"10.0.1.52","os":"RHEL 8","cpu":16,"memory":32,"disk":400,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R01-C2-02"}','手机银行,APP',NOW(),NOW()),
-- 中间件主机：ESB、Redis集群、Kafka
('ins-host-esb-01','cmdb-model-host','ESB-BUS-01','running','{"hostname":"esb-bus-01","ip":"10.0.1.71","os":"RHEL 8","cpu":16,"memory":64,"disk":500,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R02-A1-01"}','ESB',NOW(),NOW()),
('ins-host-esb-02','cmdb-model-host','ESB-BUS-02','running','{"hostname":"esb-bus-02","ip":"10.0.1.72","os":"RHEL 8","cpu":16,"memory":64,"disk":500,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R02-A1-02"}','ESB',NOW(),NOW()),
('ins-host-redis-01','cmdb-model-host','Redis-01','running','{"hostname":"redis-01","ip":"10.0.1.81","os":"CentOS 8","cpu":8,"memory":64,"disk":200,"datacenter":"SH-ZJ-MAIN","vm_type":"物理机","rack_pos":"R02-B1-01"}','缓存集群',NOW(),NOW()),
('ins-host-redis-02','cmdb-model-host','Redis-02','running','{"hostname":"redis-02","ip":"10.0.1.82","os":"CentOS 8","cpu":8,"memory":64,"disk":200,"datacenter":"SH-ZJ-MAIN","vm_type":"物理机","rack_pos":"R02-B1-02"}','缓存集群',NOW(),NOW()),
('ins-host-kafka-01','cmdb-model-host','Kafka-01','running','{"hostname":"kafka-01","ip":"10.0.1.91","os":"CentOS 8","cpu":16,"memory":64,"disk":1000,"datacenter":"SH-ZJ-MAIN","vm_type":"KVM","rack_pos":"R02-C1-01"}','消息队列',NOW(),NOW()),
-- 报表数据库
('ins-host-report-db','cmdb-model-host','报表库-DB','running','{"hostname":"report-db-01","ip":"10.0.1.120","os":"CentOS 7","cpu":16,"memory":64,"disk":2000,"datacenter":"SH-ZJ-MAIN","vm_type":"VMware","rack_pos":"R02-D1-01"}','报表库',NOW(),NOW());

-- ---- 7. 中间件实例（WebLogic、Nginx、Tomcat、Redis、Kafka、ESB-MQ）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
-- 核心 WebLogic 集群（4 节点）
('ins-mw-core-wl01','cmdb-model-middleware','核心-WLS-01','running','{"mw_type":"WebLogic","version":"12.2.1.4","port":7001,"install_path":"/opt/oracle/wls"}','WebLogic集群',NOW(),NOW()),
('ins-mw-core-wl02','cmdb-model-middleware','核心-WLS-02','running','{"mw_type":"WebLogic","version":"12.2.1.4","port":7001,"install_path":"/opt/oracle/wls"}','WebLogic集群',NOW(),NOW()),
('ins-mw-core-wl03','cmdb-model-middleware','核心-WLS-03','running','{"mw_type":"WebLogic","version":"12.2.1.4","port":7001,"install_path":"/opt/oracle/wls"}','灾备WebLogic',NOW(),NOW()),
('ins-mw-core-wl04','cmdb-model-middleware','核心-WLS-04','stopped','{"mw_type":"WebLogic","version":"12.2.1.4","port":7001,"install_path":"/opt/oracle/wls"}','灾备WebLogic',NOW(),NOW()),
-- 网银 Nginx（2 台）
('ins-mw-ebank-nginx1','cmdb-model-middleware','网银-NGINX-01','running','{"mw_type":"Nginx","version":"1.24","port":443,"install_path":"/usr/local/nginx"}','WAF前置',NOW(),NOW()),
('ins-mw-ebank-nginx2','cmdb-model-middleware','网银-NGINX-02','running','{"mw_type":"Nginx","version":"1.24","port":443,"install_path":"/usr/local/nginx"}','WAF前置',NOW(),NOW()),
-- 网银 WebLogic（2 台）
('ins-mw-ebank-wl01','cmdb-model-middleware','网银-WLS-01','running','{"mw_type":"WebLogic","version":"12.2.1.4","port":7001,"install_path":"/opt/oracle/wls"}','网银WebLogic',NOW(),NOW()),
('ins-mw-ebank-wl02','cmdb-model-middleware','网银-WLS-02','running','{"mw_type":"WebLogic","version":"12.2.1.4","port":7001,"install_path":"/opt/oracle/wls"}','网银WebLogic',NOW(),NOW()),
-- 手机银行 Tomcat（2 台）
('ins-mw-mbank-tc01','cmdb-model-middleware','手机银行-TC-01','running','{"mw_type":"Tomcat","version":"9.0.64","port":8080,"install_path":"/opt/tomcat"}','手机银行APP',NOW(),NOW()),
('ins-mw-mbank-tc02','cmdb-model-middleware','手机银行-TC-02','running','{"mw_type":"Tomcat","version":"9.0.64","port":8080,"install_path":"/opt/tomcat"}','手机银行APP',NOW(),NOW()),
-- ESB MQ（2 台）
('ins-mw-esb-mq1','cmdb-model-middleware','ESB-MQ-01','running','{"mw_type":"ActiveMQ","version":"5.18","port":61616,"install_path":"/opt/activemq"}','ESB消息总线',NOW(),NOW()),
('ins-mw-esb-mq2','cmdb-model-middleware','ESB-MQ-02','running','{"mw_type":"ActiveMQ","version":"5.18","port":61616,"install_path":"/opt/activemq"}','ESB消息总线',NOW(),NOW()),
-- Redis 集群（3 主 3 从这里先 2 节点示例）
('ins-mw-redis-01','cmdb-model-middleware','Redis-Node-01','running','{"mw_type":"Redis","version":"7.0.8","port":6379,"install_path":"/usr/local/redis"}','RedisCluster',NOW(),NOW()),
('ins-mw-redis-02','cmdb-model-middleware','Redis-Node-02','running','{"mw_type":"Redis","version":"7.0.8","port":6379,"install_path":"/usr/local/redis"}','RedisCluster',NOW(),NOW()),
-- Kafka Broker
('ins-mw-kafka-01','cmdb-model-middleware','Kafka-Broker-01','running','{"mw_type":"Kafka","version":"3.4","port":9092,"install_path":"/opt/kafka"}','消息总线',NOW(),NOW());

-- ---- 8. 数据库实例（Oracle RAC、MySQL 主从、Redis 缓存库、报表库）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-db-core-oracle1','cmdb-model-database','核心库-ORACLE-RAC1','running','{"db_type":"Oracle","version":"19c RAC","instance":"COREDB1","port":1521,"charset":"ZHS16GBK"}','核心库,RAC',NOW(),NOW()),
('ins-db-core-oracle2','cmdb-model-database','核心库-ORACLE-RAC2','running','{"db_type":"Oracle","version":"19c RAC","instance":"COREDB2","port":1521,"charset":"ZHS16GBK"}','核心库,RAC',NOW(),NOW()),
('ins-db-ebank-mysql-master','cmdb-model-database','网银库-MySQL-M','running','{"db_type":"MySQL","version":"8.0.32","instance":"ebank","port":3306,"charset":"UTF8MB4"}','网银库,主库',NOW(),NOW()),
('ins-db-ebank-mysql-slave','cmdb-model-database','网银库-MySQL-S','running','{"db_type":"MySQL","version":"8.0.32","instance":"ebank_ro","port":3306,"charset":"UTF8MB4"}','网银库,从库',NOW(),NOW()),
('ins-db-redis-cache','cmdb-model-database','Redis缓存库','running','{"db_type":"Redis","version":"7.0.8","instance":"cache-cluster","port":6379,"charset":"N/A"}','缓存库',NOW(),NOW()),
('ins-db-report-pg','cmdb-model-database','报表库-PostgreSQL','running','{"db_type":"PostgreSQL","version":"14.7","instance":"reportdb","port":5432,"charset":"UTF8"}','报表库',NOW(),NOW());

-- ---- 9. 集群（3 个：WebLogic 集群、Oracle RAC、Redis 集群）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-cl-core-wls','cmdb-model-cluster','核心系统WebLogic集群','running','{"cluster_type":"WebLogic集群","cluster_name":"CoreAppCluster","node_count":4,"ha_mode":"主备"}','核心集群',NOW(),NOW()),
('ins-cl-core-rac','cmdb-model-cluster','核心库Oracle RAC','running','{"cluster_type":"Oracle RAC","cluster_name":"CoreDB-RAC","node_count":2,"ha_mode":"双活"}','核心数据库集群',NOW(),NOW()),
('ins-cl-redis','cmdb-model-cluster','缓存Redis集群','running','{"cluster_type":"Redis集群","cluster_name":"RedisCacheCluster","node_count":2,"ha_mode":"主备"}','缓存集群',NOW(),NOW());

-- ---- 10. 业务系统（4 个：核心、网银、手机银行、ESB 渠道平台）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-bs-core','cmdb-model-business','核心系统CBS','running','{"system_code":"CBS","system_level":"1","description":"综合业务核心系统（存款/贷款/清算/计息）","rto":5,"owner":"张经理","department":"核心业务部","sec_level":"4","dr_level":"同城灾备"}','核心,P0',NOW(),NOW()),
('ins-bs-ebank','cmdb-model-business','网银系统','running','{"system_code":"EBANK","system_level":"2","description":"企业网银+个人网银","rto":30,"owner":"李经理","department":"渠道部","sec_level":"3","dr_level":"双活"}','渠道,P1',NOW(),NOW()),
('ins-bs-mbank','cmdb-model-business','手机银行APP','running','{"system_code":"MBANK","system_level":"2","description":"iOS/Android 手机银行","rto":30,"owner":"王经理","department":"渠道部","sec_level":"3","dr_level":"双活"}','渠道,P1',NOW(),NOW()),
('ins-bs-esb','cmdb-model-business','ESB企业服务总线','running','{"system_code":"ESB","system_level":"1","description":"全行服务总线，连接渠道与核心","rto":5,"owner":"赵经理","department":"架构部","sec_level":"3","dr_level":"同城灾备"}','中枢,P0',NOW(),NOW());

-- ---- 11. 批处理作业（核心日终 + 对账 + 报表）----
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at) VALUES
('ins-job-eod','cmdb-model-batch','核心日终结算','running','{"job_name":"CBS_EOD","category":"日终结算","schedule":"每日 21:00","avg_duration":180,"job_status":"启用"}','核心批处理',NOW(),NOW()),
('ins-job-recon','cmdb-model-batch','跨行对账','running','{"job_name":"INTERBANK_RECON","category":"对账","schedule":"每日 00:30","avg_duration":60,"job_status":"启用"}','对账',NOW(),NOW()),
('ins-job-report','cmdb-model-batch','监管报表生成','running','{"job_name":"REPORT_REG","category":"报表","schedule":"每日 03:00","avg_duration":90,"job_status":"启用"}','监管报送',NOW(),NOW());

-- ============================================================
-- 2. 关系拓扑（共 64 条）
-- 说明：relation_type 对应 ci_relation_types.code
-- ============================================================

-- ---- A. 机房容纳：主机/设备 运行于 数据中心（contains）----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-1', 'ins-dc01', 'ins-host-core-app1', 'contains', NOW()),
('rel-2', 'ins-dc01', 'ins-host-core-app2', 'contains', NOW()),
('rel-3', 'ins-dc01', 'ins-host-core-db1',  'contains', NOW()),
('rel-4', 'ins-dc01', 'ins-host-core-db2',  'contains', NOW()),
('rel-5', 'ins-dc01', 'ins-host-ebank-web1','contains', NOW()),
('rel-6', 'ins-dc01', 'ins-host-ebank-web2','contains', NOW()),
('rel-7', 'ins-dc01', 'ins-host-ebank-app1','contains', NOW()),
('rel-8', 'ins-dc01', 'ins-host-ebank-app2','contains', NOW()),
('rel-9', 'ins-dc01', 'ins-host-mbank-app1','contains', NOW()),
('rel-10','ins-dc01', 'ins-host-mbank-app2','contains', NOW()),
('rel-11','ins-dc01', 'ins-host-esb-01',    'contains', NOW()),
('rel-12','ins-dc01', 'ins-host-esb-02',    'contains', NOW()),
('rel-13','ins-dc01', 'ins-host-redis-01',  'contains', NOW()),
('rel-14','ins-dc01', 'ins-host-redis-02',  'contains', NOW()),
('rel-15','ins-dc01', 'ins-host-kafka-01',  'contains', NOW()),
('rel-16','ins-dc01', 'ins-host-report-db', 'contains', NOW()),
('rel-17','ins-dc01', 'ins-sw-core1',       'contains', NOW()),
('rel-18','ins-dc01', 'ins-sw-core2',       'contains', NOW()),
('rel-19','ins-dc01', 'ins-fw-zj',          'contains', NOW()),
('rel-20','ins-dc01', 'ins-san-01',         'contains', NOW()),
('rel-21','ins-dc01', 'ins-san-02',         'contains', NOW()),
('rel-22','ins-dc01', 'ins-bastion',        'contains', NOW()),
('rel-23','ins-dc01', 'ins-waf',            'contains', NOW()),
('rel-24','ins-dc01', 'ins-hsm',            'contains', NOW()),
('rel-25','ins-dc01', 'ins-f5-app',         'contains', NOW()),
-- 灾备机房
('rel-26','ins-dc02', 'ins-host-core-app3', 'contains', NOW()),
('rel-27','ins-dc02', 'ins-host-core-app4', 'contains', NOW()),
('rel-28','ins-dc02', 'ins-fw-bs',          'contains', NOW());

-- ---- B. 网络连接：核心交换机互连；机房边界防火墙连核心交换机；F5连核心交换机 ----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-n1','ins-sw-core1','ins-sw-core2',  'connects_to', NOW()),
('rel-n2','ins-fw-zj',   'ins-sw-core1',  'connects_to', NOW()),
('rel-n3','ins-fw-bs',   'ins-sw-core2',  'connects_to', NOW()),
('rel-n4','ins-f5-app',  'ins-sw-core1',  'connects_to', NOW()),
('rel-n5','ins-sw-core1','ins-san-01',    'connects_to', NOW()),
('rel-n6','ins-sw-core2','ins-san-02',    'connects_to', NOW());

-- ---- C. 主机运行于：中间件/数据库 → 主机；数据库主机 → 存储（runs_on）----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-r1','ins-mw-core-wl01','ins-host-core-app1','runs_on', NOW()),
('rel-r2','ins-mw-core-wl02','ins-host-core-app2','runs_on', NOW()),
('rel-r3','ins-mw-core-wl03','ins-host-core-app3','runs_on', NOW()),
('rel-r4','ins-mw-core-wl04','ins-host-core-app4','runs_on', NOW()),
('rel-r5','ins-mw-ebank-nginx1','ins-host-ebank-web1','runs_on', NOW()),
('rel-r6','ins-mw-ebank-nginx2','ins-host-ebank-web2','runs_on', NOW()),
('rel-r7','ins-mw-ebank-wl01','ins-host-ebank-app1','runs_on', NOW()),
('rel-r8','ins-mw-ebank-wl02','ins-host-ebank-app2','runs_on', NOW()),
('rel-r9','ins-mw-mbank-tc01','ins-host-mbank-app1','runs_on', NOW()),
('rel-r10','ins-mw-mbank-tc02','ins-host-mbank-app2','runs_on', NOW()),
('rel-r11','ins-mw-esb-mq1','ins-host-esb-01','runs_on', NOW()),
('rel-r12','ins-mw-esb-mq2','ins-host-esb-02','runs_on', NOW()),
('rel-r13','ins-mw-redis-01','ins-host-redis-01','runs_on', NOW()),
('rel-r14','ins-mw-redis-02','ins-host-redis-02','runs_on', NOW()),
('rel-r15','ins-mw-kafka-01','ins-host-kafka-01','runs_on', NOW()),
('rel-r16','ins-db-core-oracle1','ins-host-core-db1','runs_on', NOW()),
('rel-r17','ins-db-core-oracle2','ins-host-core-db2','runs_on', NOW()),
('rel-r18','ins-db-ebank-mysql-master','ins-host-ebank-app1','runs_on', NOW()),
('rel-r19','ins-db-ebank-mysql-slave','ins-host-ebank-app2','runs_on', NOW()),
('rel-r20','ins-db-redis-cache','ins-host-redis-01','runs_on', NOW()),
('rel-r21','ins-db-report-pg','ins-host-report-db','runs_on', NOW()),
-- 存储挂载
('rel-r22','ins-host-core-db1','ins-san-01','runs_on', NOW()),
('rel-r23','ins-host-core-db2','ins-san-01','runs_on', NOW()),
('rel-r24','ins-host-report-db','ins-san-02','runs_on', NOW());

-- ---- D. 集群包含：中间件/数据库 属于某个集群（contains/manages）----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-c1','ins-cl-core-wls','ins-mw-core-wl01','contains', NOW()),
('rel-c2','ins-cl-core-wls','ins-mw-core-wl02','contains', NOW()),
('rel-c3','ins-cl-core-wls','ins-mw-core-wl03','contains', NOW()),
('rel-c4','ins-cl-core-wls','ins-mw-core-wl04','contains', NOW()),
('rel-c5','ins-cl-core-rac','ins-db-core-oracle1','manages', NOW()),
('rel-c6','ins-cl-core-rac','ins-db-core-oracle2','manages', NOW()),
('rel-c7','ins-cl-redis','ins-mw-redis-01','contains', NOW()),
('rel-c8','ins-cl-redis','ins-mw-redis-02','contains', NOW());

-- ---- E. 业务系统依赖链 ----
-- 业务系统 → 集群（depends_on）
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-b1','ins-bs-core','ins-cl-core-wls','depends_on', NOW()),
('rel-b2','ins-bs-core','ins-cl-core-rac','depends_on', NOW()),
('rel-b3','ins-bs-ebank','ins-mw-ebank-nginx1','depends_on', NOW()),
('rel-b4','ins-bs-ebank','ins-mw-ebank-nginx2','depends_on', NOW()),
('rel-b5','ins-bs-ebank','ins-mw-ebank-wl01','depends_on', NOW()),
('rel-b6','ins-bs-ebank','ins-mw-ebank-wl02','depends_on', NOW()),
('rel-b7','ins-bs-ebank','ins-db-ebank-mysql-master','depends_on', NOW()),
('rel-b8','ins-bs-mbank','ins-mw-mbank-tc01','depends_on', NOW()),
('rel-b9','ins-bs-mbank','ins-mw-mbank-tc02','depends_on', NOW()),
('rel-b10','ins-bs-esb','ins-mw-esb-mq1','depends_on', NOW()),
('rel-b11','ins-bs-esb','ins-mw-esb-mq2','depends_on', NOW()),
('rel-b12','ins-bs-esb','ins-mw-kafka-01','depends_on', NOW()),
('rel-b13','ins-bs-esb','ins-cl-redis','depends_on', NOW()),
-- 跨系统依赖：渠道 → ESB → 核心
('rel-b14','ins-bs-ebank','ins-bs-esb','depends_on', NOW()),
('rel-b15','ins-bs-mbank','ins-bs-esb','depends_on', NOW()),
('rel-b16','ins-bs-esb','ins-bs-core','depends_on', NOW());

-- ---- F. 负载均衡后端池：F5 contains VIP；F5 contains 后端成员 ----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-f1','ins-f5-app','ins-vip-core', 'contains', NOW()),
('rel-f2','ins-f5-app','ins-vip-ebank','contains', NOW()),
('rel-f3','ins-f5-app','ins-vip-mbank','contains', NOW()),
('rel-f4','ins-f5-app','ins-vip-ecs',  'contains', NOW()),
('rel-f5','ins-f5-app','ins-mw-core-wl01','contains', NOW()),
('rel-f6','ins-f5-app','ins-mw-core-wl02','contains', NOW()),
('rel-f7','ins-f5-app','ins-mw-ebank-nginx1','contains', NOW()),
('rel-f8','ins-f5-app','ins-mw-ebank-nginx2','contains', NOW()),
('rel-f9','ins-f5-app','ins-mw-mbank-tc01','contains', NOW()),
('rel-f10','ins-f5-app','ins-mw-mbank-tc02','contains', NOW()),
('rel-f11','ins-f5-app','ins-mw-esb-mq1','contains', NOW()),
('rel-f12','ins-f5-app','ins-mw-esb-mq2','contains', NOW());

-- ---- G. 安全：WAF/堡垒机/加密机 接入 ----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-s1','ins-waf','ins-vip-ebank','monitors', NOW()),
('rel-s2','ins-waf','ins-vip-mbank','monitors', NOW()),
('rel-s3','ins-bastion','ins-host-core-app1','manages', NOW()),
('rel-s4','ins-bastion','ins-host-core-db1','manages', NOW()),
-- Oracle TDE 透明加密/签名校验需调用加密机，所以 数据库 depends_on 加密机
('rel-s5','ins-db-core-oracle1','ins-hsm','depends_on', NOW()),
('rel-s6','ins-db-core-oracle2','ins-hsm','depends_on', NOW());

-- ---- H. 备份/同步：数据库主从、灾备 ----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
-- MySQL 主 → 从 同步
('rel-d1','ins-db-ebank-mysql-master','ins-db-ebank-mysql-slave','syncs_to', NOW()),
-- 灾备应用 备份 主中心应用
('rel-d2','ins-host-core-app3','ins-host-core-app1','backs_up', NOW()),
('rel-d3','ins-host-core-app4','ins-host-core-app2','backs_up', NOW()),
-- 宝山防火墙 备份 张江防火墙
('rel-d4','ins-fw-bs','ins-fw-zj','backs_up', NOW()),
-- 报表库 同步 核心库数据
('rel-d5','ins-db-core-oracle1','ins-db-report-pg','syncs_to', NOW());

-- ---- I. 批处理：作业依赖业务系统 ----
INSERT INTO ci_relations (id, source_id, target_id, relation_type, created_at) VALUES
('rel-j1','ins-job-eod',   'ins-bs-core','depends_on', NOW()),
('rel-j2','ins-job-recon', 'ins-bs-core','depends_on', NOW()),
('rel-j3','ins-job-recon', 'ins-bs-esb','depends_on',  NOW()),
('rel-j4','ins-job-report','ins-db-report-pg','depends_on', NOW());
