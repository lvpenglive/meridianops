#!/usr/bin/env node
/**
 * MeridianOps License 签发工具
 *
 * 用法:
 *   node tools/issue_license.js --edition Enterprise --customer "XX银行" --expires 2027-12-31
 *   node tools/issue_license.js --edition Ultimate --customer "XX银行" --days 365 --fingerprint abc123def456
 *   node tools/issue_license.js --edition Community --customer "测试" --perpetual
 *
 * 参数:
 *   --edition      Community | Enterprise | Ultimate （必填）
 *   --customer     客户名称 （必填）
 *   --expires      到期日期 YYYY-MM-DD 或 YYYY-MM-DD HH:MM:SS （与 --days/--perpetual 三选一）
 *   --days         授权天数（从签发时刻算起），0 或不传 + --perpetual = 永久
 *   --perpetual    永不到期
 *   --fingerprint  绑定机器指纹（从授权管理页获取，留空 = 通用激活码）
 *
 * 输出:
 *   激活码字符串（以 MK- 开头），可直接粘贴到授权管理页的「激活码」输入框
 */

const crypto = require('crypto');

// ===== 私钥（对应后端内置的 Ed25519 公钥）=====
// 注意：此私钥应妥善保管，仅授权签发人员持有！
const LICENSE_PRIVATE_KEY_HEX = '2753cc23a0eb4002f5b3dad804d06d98e5a3c1d84d951b4c89e27893530ed86c';

// ===== 参数解析 =====
function parseArgs() {
  const args = {};
  const argv = process.argv.slice(2);
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith('--')) {
      const key = argv[i].slice(2);
      const val = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[i + 1] : true;
      args[key] = val;
      if (val !== true) i++;
    }
  }
  return args;
}

// ===== Base64URL 编码（无填充）=====
function base64url(buf) {
  return buf.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

// ===== 主逻辑 =====
function main() {
  const args = parseArgs();

  const edition = args.edition;
  const customer = args.customer;
  if (!edition || !customer) {
    console.error('错误: --edition 和 --customer 为必填项');
    process.exit(1);
  }

  if (!['Community', 'Enterprise', 'Ultimate'].includes(edition)) {
    console.error('错误: --edition 只能是 Community / Enterprise / Ultimate');
    process.exit(1);
  }

  // 计算到期时间
  let expiresAt = '';
  let validDays = 0;
  const now = new Date();

  if (args.perpetual) {
    // 永不到期
    expiresAt = '';
    validDays = 0;
  } else if (args.days) {
    // 按天数
    validDays = parseInt(args.days, 10);
    const exp = new Date(now.getTime() + validDays * 24 * 60 * 60 * 1000);
    expiresAt = exp.toISOString();
  } else if (args.expires) {
    // 按日期
    let exp;
    if (args.expires.includes(':')) {
      exp = new Date(args.expires);
    } else {
      // 仅日期，设为当天 23:59:59 UTC
      exp = new Date(args.expires + 'T23:59:59Z');
    }
    if (isNaN(exp.getTime())) {
      console.error('错误: --expires 日期格式无效');
      process.exit(1);
    }
    expiresAt = exp.toISOString();
    validDays = Math.ceil((exp.getTime() - now.getTime()) / (24 * 60 * 60 * 1000));
  } else {
    console.error('错误: 需要指定 --expires / --days / --perpetual 之一');
    process.exit(1);
  }

  // 构建 payload
  const payload = {
    edition: edition,
    customer: customer,
    expiresAt: expiresAt,
    fingerprint: args.fingerprint || '',
    issuedAt: now.toISOString(),
    validDays: validDays,
  };

  // 序列化 payload
  const payloadJson = JSON.stringify(payload);
  const payloadBytes = Buffer.from(payloadJson, 'utf8');
  const payloadB64 = base64url(payloadBytes);

  // 加载私钥并签名
  // Ed25519 raw 32 bytes -> PKCS8 DER 包装
  // PKCS8 header for Ed25519: 30 2e 02 01 00 30 05 06 03 2b 65 70 04 22 04 20
  const privKeyRaw = Buffer.from(LICENSE_PRIVATE_KEY_HEX, 'hex');
  const pkcs8Prefix = Buffer.from([
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20
  ]);
  const pkcs8Der = Buffer.concat([pkcs8Prefix, privKeyRaw]);
  const privateKey = crypto.createPrivateKey({ key: pkcs8Der, format: 'der', type: 'pkcs8' });

  // 签名（Ed25519 签名的是原始 JSON 字节，与 Rust 端 verify 一致）
  const sig = crypto.sign(null, payloadBytes, privateKey);
  const sigB64 = base64url(sig);

  // 组装激活码
  const licenseKey = `MK-${payloadB64}.${sigB64}`;

  // 输出
  console.log('========== MeridianOps License 签发工具 ==========');
  console.log('');
  console.log('--- 授权信息 ---');
  console.log(`  版本:     ${edition}`);
  console.log(`  客户:     ${customer}`);
  console.log(`  到期时间: ${expiresAt || '永不到期'}`);
  console.log(`  有效天数: ${validDays || '永久'}`);
  console.log(`  机器指纹: ${args.fingerprint || '不绑定（通用激活码）'}`);
  console.log(`  签发时间: ${now.toISOString()}`);
  console.log('');
  console.log('--- 激活码 ---');
  console.log(licenseKey);
  console.log('');
  console.log('使用方法: 将上述激活码粘贴到「授权管理 → 更新授权 → 激活码」输入框中。');
}

main();
