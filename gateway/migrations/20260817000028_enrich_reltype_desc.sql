-- 20260817000028: 丰富关系类型描述，补充银行业运维场景说明

UPDATE ci_relation_types SET description = 'A 依赖 B 才能正常运行。B 故障将直接导致 A 不可用。
典型场景：核心交易系统依赖数据库服务、支付网关依赖加密机(HSM)、前端应用依赖 Redis 缓存。
用途：影响分析——当 B 发生故障或维护时，通过此关系可快速定位所有受影响的 A 资产，评估变更风险范围。' WHERE id = 'reltype-depends-on';

UPDATE ci_relation_types SET description = 'A 包含 B 作为子组件或逻辑子单元。B 是 A 的组成部分。
典型场景：WebLogic 集群包含多个 Server 节点、核心系统包含清算/结算/报表子系统、F5 负载均衡包含多个 VIP。
用途：层级展示——在拓扑视图中展开父资产可查看其子组件清单，支持从集群→节点→实例的逐层下钻。' WHERE id = 'reltype-contains';

UPDATE ci_relation_types SET description = 'A 运行在 B 之上，B 为 A 提供运行环境或承载平台。
典型场景：核心交易应用运行于 Linux 主机、WebLogic 实例运行于中间件主机、MySQL 运行于数据库服务器。
用途：部署映射——从应用视角定位其宿主主机，便于执行运维作业（巡检、补丁、日志清理）时自动选择目标范围。' WHERE id = 'reltype-runs-on';

UPDATE ci_relation_types SET description = 'A 管理 B，A 是 B 的管理控制节点。
典型场景：WebLogic Admin Server 管理多个 Managed Server、K8s Master 管理集群节点、Ansible 控制端管理目标主机。
用途：控制关系——识别资产的管理入口，当管理节点故障时可评估受管资产是否失去管控能力。' WHERE id = 'reltype-manages';

UPDATE ci_relation_types SET description = 'A 与 B 之间存在网络连接或通信链路（无向关系，双向对称）。
典型场景：核心交换机互联、数据库与中间件之间 TCP 连接、跨数据中心专线连接、F5 与后端服务器网络可达。
用途：网络拓扑——绘制网络连通关系图，辅助网络故障排查和变更影响分析。' WHERE id = 'reltype-connects-to';

UPDATE ci_relation_types SET description = 'A 是 B 的备份实例，B 故障时 A 可接管服务（主备/双活容灾）。
典型场景：MySQL 从库备份主库、Oracle Data Guard 备库、同城灾备中心备份生产中心、备 DNS 服务器。
用途：容灾评估——梳理主备关系链路，验证 RPO/RTO 覆盖完整性，确保关键业务有足够备份能力。' WHERE id = 'reltype-backs-up';

UPDATE ci_relation_types SET description = 'A 同步数据至 B，数据从 A 单向复制到 B（区别于备份，强调数据流动）。
典型场景：MySQL 主从复制、Oracle GoldenGate 数据同步、跨数据中心数据复制、CDC 增量同步到数仓。
用途：数据流分析——追踪数据同步链路，当源端 A 变更时评估下游 B 的影响，排查数据一致性问题和同步延迟。' WHERE id = 'reltype-syncs-to';

UPDATE ci_relation_types SET description = 'A 监控 B 的运行状态和健康指标。
典型场景：Zabbix Server 监控主机和中间件、Prometheus 监控容器和应用、日志采集 Agent 监控目标主机。
用途：监控覆盖——检查哪些资产已被监控、哪些存在监控盲区，确保关键资产 100% 纳入监控体系。' WHERE id = 'reltype-monitors';
