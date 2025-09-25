#!/usr/bin/env node
/**
 * Build environment validation script for Manifest
 * Ensures all dependencies and tools are available
 */

import { execSync } from 'child_process';
import { existsSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

interface ValidationResult {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message: string;
  required: boolean;
}

class BuildValidator {
  private readonly ROOT_DIR = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  private results: ValidationResult[] = [];

  async validate(): Promise<boolean> {
    console.log('🔍 Validating build environment...\n');

    this.checkRust();
    this.checkNode();
    this.checkTauriCli();
    this.checkIcons();
    this.checkFrontendDeps();
    this.checkPlatformTools();

    this.printResults();
    return this.results.filter(r => r.required && r.status === 'fail').length === 0;
  }

  private checkRust(): void {
    try {
      const version = execSync('rustc --version', { encoding: 'utf8' });
      this.addResult('Rust Compiler', 'pass', `Found: ${version.trim()}`, true);
      
      const cargoVersion = execSync('cargo --version', { encoding: 'utf8' });
      this.addResult('Cargo', 'pass', `Found: ${cargoVersion.trim()}`, true);
    } catch {
      this.addResult('Rust/Cargo', 'fail', 'Rust toolchain not found. Install from https://rustup.rs/', true);
    }
  }

  private checkNode(): void {
    try {
      const version = execSync('node --version', { encoding: 'utf8' });
      const major = parseInt(version.slice(1).split('.')[0]);
      
      if (major >= 18) {
        this.addResult('Node.js', 'pass', `Found: ${version.trim()}`, true);
      } else {
        this.addResult('Node.js', 'fail', `Version ${version.trim()} too old. Requires Node 18+`, true);
      }
      
      execSync('npm --version', { encoding: 'utf8' });
      this.addResult('NPM', 'pass', 'NPM available', true);
    } catch {
      this.addResult('Node.js/NPM', 'fail', 'Node.js not found. Install from https://nodejs.org/', true);
    }
  }

  private checkTauriCli(): void {
    try {
      const version = execSync('cargo tauri --version', { encoding: 'utf8', cwd: this.ROOT_DIR });
      this.addResult('Tauri CLI', 'pass', `Found: ${version.trim()}`, true);
    } catch {
      this.addResult('Tauri CLI', 'fail', 'Install with: cargo install tauri-cli --version "^2.0"', true);
    }
  }

  private checkIcons(): void {
    const iconDir = join(this.ROOT_DIR, 'backend/icons');
    const requiredIcons = [
      '32x32.png',
      '128x128.png',
      '128x128@2x.png',
      'icon.ico',
      'icon.icns'
    ];

    let missingIcons = 0;
    requiredIcons.forEach(icon => {
      if (!existsSync(join(iconDir, icon))) {
        missingIcons++;
      }
    });

    if (missingIcons === 0) {
      this.addResult('App Icons', 'pass', 'All required icons found', true);
    } else {
      this.addResult('App Icons', 'fail', `Missing ${missingIcons} required icons`, true);
    }
  }

  private checkFrontendDeps(): void {
    const frontendDir = join(this.ROOT_DIR, 'frontend');
    const nodeModules = join(frontendDir, 'node_modules');
    const distDir = join(frontendDir, 'dist');

    if (!existsSync(nodeModules)) {
      this.addResult('Frontend Dependencies', 'fail', 'Run npm install in frontend/', true);
    } else {
      this.addResult('Frontend Dependencies', 'pass', 'Dependencies installed', true);
    }

    if (!existsSync(distDir)) {
      this.addResult('Frontend Build', 'warn', 'Frontend not built. Will build automatically.', false);
    } else {
      this.addResult('Frontend Build', 'pass', 'Frontend build exists', false);
    }
  }

  private checkPlatformTools(): void {
    const platform = process.platform;

    if (platform === 'darwin') {
      // macOS-specific tools
      try {
        execSync('which codesign', { encoding: 'utf8' });
        this.addResult('macOS Code Signing', 'pass', 'codesign available', false);
      } catch {
        this.addResult('macOS Code Signing', 'warn', 'codesign not found (needed for distribution)', false);
      }
    }

    if (platform === 'win32') {
      // Windows-specific tools
      try {
        execSync('where signtool', { encoding: 'utf8' });
        this.addResult('Windows Signing', 'pass', 'signtool available', false);
      } catch {
        this.addResult('Windows Signing', 'warn', 'signtool not found (needed for distribution)', false);
      }
    }

    if (platform === 'linux') {
      // Linux-specific tools
      try {
        execSync('which dpkg-deb', { encoding: 'utf8' });
        this.addResult('Linux Packaging', 'pass', 'dpkg-deb available', false);
      } catch {
        this.addResult('Linux Packaging', 'warn', 'dpkg-deb not found (needed for .deb packages)', false);
      }
    }
  }

  private addResult(name: string, status: 'pass' | 'fail' | 'warn', message: string, required: boolean): void {
    this.results.push({ name, status, message, required });
  }

  private printResults(): void {
    console.log('📋 Validation Results:\n');

    const maxNameLength = Math.max(...this.results.map(r => r.name.length));

    this.results.forEach(result => {
      const icon = result.status === 'pass' ? '✅' : result.status === 'fail' ? '❌' : '⚠️';
      const name = result.name.padEnd(maxNameLength);
      const required = result.required ? '[REQUIRED]' : '[OPTIONAL]';
      
      console.log(`${icon} ${name} ${required.padEnd(12)} ${result.message}`);
    });

    const failures = this.results.filter(r => r.required && r.status === 'fail');
    const warnings = this.results.filter(r => r.status === 'warn');

    console.log(`\n📊 Summary:`);
    console.log(`   ✅ Passed: ${this.results.filter(r => r.status === 'pass').length}`);
    console.log(`   ❌ Failed: ${failures.length}`);
    console.log(`   ⚠️  Warnings: ${warnings.length}`);

    if (failures.length > 0) {
      console.log('\n❌ Build environment not ready. Fix the required failures above.');
    } else {
      console.log('\n✅ Build environment ready!');
    }
  }
}

// Main execution
const isMainModule = import.meta.url === `file://${process.argv[1]}`;
if (isMainModule) {
  const validator = new BuildValidator();
  validator.validate()
    .then(success => {
      process.exit(success ? 0 : 1);
    })
    .catch(console.error);
}

export { BuildValidator };
