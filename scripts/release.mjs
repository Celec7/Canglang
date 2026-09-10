#!/usr/bin/env node

/**
 * 沧浪 (Canglang) 自动化版本发布与 Changelog 准备脚本
 *
 * 用法:
 *   node scripts/release.mjs [patch|minor|major|<自定义版本号>] [选项]
 *
 * 选项:
 *   --dry-run       仅预览变更，不写盘且不产生 git commit / tag
 *   --no-commit     仅更新版本文件与 Changelog，不执行 git commit 与 tag
 *   --allow-dirty   允许工作区存在未提交改动时执行
 *
 * 示例:
 *   node scripts/release.mjs patch
 *   node scripts/release.mjs 0.2.0
 *   node scripts/release.mjs minor --dry-run
 */

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { execSync } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = resolve(__dirname, "..");

// 辅助终端高亮
const c = {
  reset: "\x1b[0m",
  bold: "\x1b[1m",
  dim: "\x1b[2m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
  red: "\x1b[31m",
};

function run(command, options = {}) {
  return execSync(command, {
    cwd: rootDir,
    encoding: "utf-8",
    stdio: options.silent ? "pipe" : "inherit",
    ...options,
  });
}

function runCapture(command) {
  try {
    return execSync(command, { cwd: rootDir, encoding: "utf-8", stdio: ["pipe", "pipe", "ignore"] }).trim();
  } catch {
    return "";
  }
}

function parseSemVer(versionStr) {
  const match = versionStr.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/);
  if (!match) return null;
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
    prerelease: match[4] || null,
  };
}

function computeNextVersion(currentVersion, bumpTypeOrVersion) {
  if (["patch", "minor", "major"].includes(bumpTypeOrVersion)) {
    const sem = parseSemVer(currentVersion);
    if (!sem) {
      throw new Error(`当前版本号 "${currentVersion}" 无法解析为有效 SemVer`);
    }
    if (bumpTypeOrVersion === "patch") {
      return `${sem.major}.${sem.minor}.${sem.patch + 1}`;
    } else if (bumpTypeOrVersion === "minor") {
      return `${sem.major}.${sem.minor + 1}.0`;
    } else if (bumpTypeOrVersion === "major") {
      return `${sem.major + 1}.0.0`;
    }
  }

  // 显式指定的版本号
  const custom = bumpTypeOrVersion.replace(/^v/, "");
  if (!parseSemVer(custom)) {
    throw new Error(`指定的版本号 "${custom}" 不符合 SemVer 格式 (例如 0.2.0 或 1.0.0-rc.1)`);
  }
  return custom;
}

function checkGitStatus(allowDirty) {
  const status = runCapture("git status --porcelain");
  if (status && !allowDirty) {
    console.error(`${c.red}❌ 错误: Git 工作区存在未提交的修改！${c.reset}`);
    console.error(c.dim + status + c.reset);
    console.error(`请先提交或使用 git stash 保存修改，或使用 ${c.cyan}--allow-dirty${c.reset} 选项强制运行。`);
    process.exit(1);
  }
}

function main() {
  const args = process.argv.slice(2);
  const positionalArgs = args.filter((a) => !a.startsWith("--"));
  const isDryRun = args.includes("--dry-run");
  const noCommit = args.includes("--no-commit");
  const allowDirty = args.includes("--allow-dirty");

  const targetArg = positionalArgs[0];
  if (!targetArg) {
    console.log(`
${c.bold}沧浪 (Canglang) 发布版本准备脚本${c.reset}

${c.cyan}用法:${c.reset}
  pnpm release [patch | minor | major | <版本号>] [选项]

${c.cyan}示例:${c.reset}
  pnpm release patch              # 0.1.0 -> 0.1.1
  pnpm release minor              # 0.1.0 -> 0.2.0
  pnpm release 0.2.0              # 自定义指定版本为 0.2.0
  pnpm release patch --dry-run    # 仅预览，不修改任何文件

${c.cyan}选项:${c.reset}
  --dry-run       仅演练，不修改文件与产生 Git 提交
  --no-commit     仅更新版本和日志，不产生 Git 提交与 Tag
  --allow-dirty   允许在存在未提交改动时执行
`);
    process.exit(0);
  }

  console.log(`\n${c.bold}${c.blue}=== 沧浪 (Canglang) 发版准备流水线 ===${c.reset}`);
  if (isDryRun) {
    console.log(`${c.yellow}⚠️  [DRY-RUN] 预览模式开启，不会对文件和 Git 进行实际更改${c.reset}\n`);
  }

  // 1. 安全检查
  if (!isDryRun) {
    checkGitStatus(allowDirty);
  }

  // 2. 读取当前版本
  const pkgPath = resolve(rootDir, "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf-8"));
  const currentVersion = pkg.version;
  const nextVersion = computeNextVersion(currentVersion, targetArg);
  const nextTag = `v${nextVersion}`;

  console.log(`📌 版本变更: ${c.yellow}v${currentVersion}${c.reset} ──> ${c.green}${nextTag}${c.reset} (${targetArg})\n`);

  // 3. 文件列表与路径准备
  const cargoTomlPath = resolve(rootDir, "src-tauri/Cargo.toml");
  const tauriConfPath = resolve(rootDir, "src-tauri/tauri.conf.json");
  const changelogPath = resolve(rootDir, "CHANGELOG.md");

  // 更新 package.json
  pkg.version = nextVersion;
  const updatedPkgContent = JSON.stringify(pkg, null, 2) + "\n";

  // 更新 src-tauri/Cargo.toml
  const cargoTomlRaw = readFileSync(cargoTomlPath, "utf-8");
  const updatedCargoToml = cargoTomlRaw.replace(
    /(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
    `$1"${nextVersion}"`
  );

  // 更新 src-tauri/tauri.conf.json
  const tauriConfRaw = readFileSync(tauriConfPath, "utf-8");
  const tauriConf = JSON.parse(tauriConfRaw);
  tauriConf.version = nextVersion;
  const updatedTauriConf = JSON.stringify(tauriConf, null, 2) + "\n";

  console.log(`${c.cyan}[1/4] 同步更新项目版本配置...${c.reset}`);
  console.log(`  ✔ package.json -> ${nextVersion}`);
  console.log(`  ✔ src-tauri/Cargo.toml -> ${nextVersion}`);
  console.log(`  ✔ src-tauri/tauri.conf.json -> ${nextVersion}`);

  if (!isDryRun) {
    writeFileSync(pkgPath, updatedPkgContent);
    writeFileSync(cargoTomlPath, updatedCargoToml);
    writeFileSync(tauriConfPath, updatedTauriConf);
    // 运行 cargo check 刷新 Cargo.lock
    console.log(`  ✔ 运行 cargo check 刷新 Cargo.lock...`);
    run("cargo check --quiet", { stdio: "ignore" });
  }

  // 4. 更新 Changelog
  console.log(`\n${c.cyan}[2/4] 生成并追加更新日志 (Changelog)...${c.reset}`);
  const hasGitCliff = runCapture("which git-cliff");
  const today = new Date().toISOString().slice(0, 10);

  if (hasGitCliff) {
    console.log(`  ✔ 检测到系统已安装 git-cliff，调用官方引擎...`);
    if (!isDryRun) {
      run(`git-cliff --tag ${nextTag} --output CHANGELOG.md`);
    } else {
      console.log(`  [DRY-RUN] 将执行: git-cliff --tag ${nextTag} --output CHANGELOG.md`);
    }
  } else {
    console.log(`  ℹ 未检测到系统 git-cliff，采用内置 Conventional Commits 解析回退...`);
    const lastTag = runCapture("git describe --tags --abbrev=0") || "";
    const logRange = lastTag ? `${lastTag}..HEAD` : "HEAD";
    const commitLogs = runCapture(`git log ${logRange} --pretty=format:"%h%x09%s"`);

    const groups = {
      feat: [],
      fix: [],
      perf: [],
      docs: [],
      refactor: [],
      other: [],
    };

    if (commitLogs) {
      for (const line of commitLogs.split("\n")) {
        const [hash, ...rest] = line.split("\t");
        const msg = rest.join("\t").trim();
        if (!msg || msg.startsWith("chore(release)")) continue;

        if (msg.startsWith("feat")) groups.feat.push({ hash, msg });
        else if (msg.startsWith("fix")) groups.fix.push({ hash, msg });
        else if (msg.startsWith("perf")) groups.perf.push({ hash, msg });
        else if (msg.startsWith("docs")) groups.docs.push({ hash, msg });
        else if (msg.startsWith("refactor")) groups.refactor.push({ hash, msg });
        else groups.other.push({ hash, msg });
      }
    }

    let releaseSection = `\n## [${nextVersion}] - ${today}\n\n`;
    if (groups.feat.length > 0) {
      releaseSection += `### ✨ 新增功能 (Features)\n`;
      for (const item of groups.feat) releaseSection += `- ${item.msg} (${item.hash})\n`;
      releaseSection += "\n";
    }
    if (groups.fix.length > 0) {
      releaseSection += `### 🐛 缺陷修复 (Bug Fixes)\n`;
      for (const item of groups.fix) releaseSection += `- ${item.msg} (${item.hash})\n`;
      releaseSection += "\n";
    }
    if (groups.perf.length > 0) {
      releaseSection += `### ⚡ 性能优化 (Performance)\n`;
      for (const item of groups.perf) releaseSection += `- ${item.msg} (${item.hash})\n`;
      releaseSection += "\n";
    }
    if (groups.docs.length > 0) {
      releaseSection += `### 📝 文档更新 (Documentation)\n`;
      for (const item of groups.docs) releaseSection += `- ${item.msg} (${item.hash})\n`;
      releaseSection += "\n";
    }
    if (groups.refactor.length > 0) {
      releaseSection += `### ♻️ 代码重构 (Refactor)\n`;
      for (const item of groups.refactor) releaseSection += `- ${item.msg} (${item.hash})\n`;
      releaseSection += "\n";
    }
    if (groups.other.length > 0) {
      releaseSection += `### 🔧 其他变更\n`;
      for (const item of groups.other) releaseSection += `- ${item.msg} (${item.hash})\n`;
      releaseSection += "\n";
    }

    if (existsSync(changelogPath)) {
      const oldChangelog = readFileSync(changelogPath, "utf-8");
      // 在第一处 "## [" 前插入新版本章节
      const insertIdx = oldChangelog.indexOf("## [");
      const newChangelog =
        insertIdx !== -1
          ? oldChangelog.slice(0, insertIdx) + releaseSection.trim() + "\n\n" + oldChangelog.slice(insertIdx)
          : oldChangelog + "\n" + releaseSection;

      if (!isDryRun) {
        writeFileSync(changelogPath, newChangelog);
        console.log(`  ✔ 已追加更新内容到 CHANGELOG.md`);
      } else {
        console.log(`  [DRY-RUN] 生成的更新内容预览:\n${releaseSection}`);
      }
    }
  }

  // 5. Git 提交与 Tag
  console.log(`\n${c.cyan}[3/4] 状态与提交处理...${c.reset}`);
  if (isDryRun || noCommit) {
    console.log(`  ℹ 跳过 Git commit 与 tag (${isDryRun ? "dry-run 模式" : "--no-commit 设置"})`);
  } else {
    run("git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock CHANGELOG.md");
    if (existsSync(resolve(rootDir, "src-tauri/Cargo.lock"))) {
      run("git add src-tauri/Cargo.lock");
    }
    const commitMsg = `chore(release): bump version to ${nextVersion}`;
    run(`git commit -m "${commitMsg}"`);
    console.log(`  ✔ 已提交: "${commitMsg}"`);
    run(`git tag -a ${nextTag} -m "Release ${nextTag}"`);
    console.log(`  ✔ 已打标签: ${nextTag}`);
  }

  // 6. 成功与下一步指引
  console.log(`\n${c.bold}${c.green}🎉 [4/4] 本地发版准备完成！${c.reset}`);
  if (!isDryRun && !noCommit) {
    console.log(`\n推送到远程以触发全自动化构建与 GitHub Release：`);
    console.log(`  ${c.bold}${c.cyan}git push && git push origin ${nextTag}${c.reset}\n`);
  }
}

try {
  main();
} catch (err) {
  console.error(`\n${c.red}❌ 执行失败: ${err.message}${c.reset}`);
  process.exit(1);
}
