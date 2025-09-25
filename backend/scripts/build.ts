#!/usr/bin/env node
/**
 * Cross-platform build script for Manifest
 * Supports: Windows (MSI), macOS (DMG/APP), Linux (DEB/AppImage)
 */

import { execSync } from 'child_process';
import { existsSync, readFileSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

type Platform = 'windows' | 'macos' | 'linux' | 'all';
type BuildType = 'dev' | 'release';
type Target = 'msi' | 'dmg' | 'app' | 'deb' | 'appimage';

interface BuildConfig {
  platform: Platform;
  buildType: BuildType;
  targets?: Target[];
  verbose?: boolean;
  skipFrontend?: boolean;
}

class ManifestBuilder {
  private readonly ROOT_DIR = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  private readonly BACKEND_DIR = join(this.ROOT_DIR, 'backend');
  private readonly FRONTEND_DIR = join(this.ROOT_DIR, 'frontend');

  private readonly PLATFORM_TARGETS: Record<Platform, Target[]> = {
    windows: ['msi'],
    macos: ['dmg', 'app'],
    linux: ['deb', 'appimage'],
    all: ['msi', 'dmg', 'app', 'deb', 'appimage']
  };

  constructor(private config: BuildConfig) {
    this.validateConfig();
  }

  async build(): Promise<void> {
    console.log(`🚀 Building Manifest for ${this.config.platform}...`);
    
    try {
      if (!this.config.skipFrontend) {
        await this.buildFrontend();
      }
      
      await this.buildBackend();
      console.log('✅ Build completed successfully!');
    } catch (error) {
      console.error('❌ Build failed:', error);
      process.exit(1);
    }
  }

  private validateConfig(): void {
    if (!existsSync(this.BACKEND_DIR)) {
      throw new Error('Backend directory not found');
    }
    if (!existsSync(this.FRONTEND_DIR)) {
      throw new Error('Frontend directory not found');
    }
  }

  private async buildFrontend(): Promise<void> {
    console.log('📦 Building frontend...');
    
    try {
      execSync('npm run build', { 
        cwd: this.FRONTEND_DIR, 
        stdio: this.config.verbose ? 'inherit' : 'pipe' 
      });
      console.log('✅ Frontend build completed');
    } catch (error) {
      throw new Error(`Frontend build failed: ${error}`);
    }
  }

  private async buildBackend(): Promise<void> {
    console.log('⚙️  Building backend and creating installers...');
    
    const targets = this.config.targets || this.PLATFORM_TARGETS[this.config.platform];
    const buildFlags = this.getBuildFlags();
    
    for (const target of targets) {
      console.log(`📋 Creating ${target.toUpperCase()} installer...`);
      
      try {
        const command = `cargo tauri build ${buildFlags} --target ${target}`;
        execSync(command, { 
          cwd: this.BACKEND_DIR, 
          stdio: this.config.verbose ? 'inherit' : 'pipe' 
        });
        console.log(`✅ ${target.toUpperCase()} installer created`);
      } catch (error) {
        console.warn(`⚠️  Failed to create ${target} installer:`, error);
      }
    }
  }

  private getBuildFlags(): string {
    const flags: string[] = [];
    
    if (this.config.buildType === 'release') {
      flags.push('--release');
    }
    
    if (this.config.verbose) {
      flags.push('--verbose');
    }
    
    return flags.join(' ');
  }

  static parseArgs(): BuildConfig {
    const args = process.argv.slice(2);
    const config: BuildConfig = {
      platform: 'all',
      buildType: 'release',
      verbose: false,
      skipFrontend: false
    };

    for (let i = 0; i < args.length; i++) {
      switch (args[i]) {
        case '--platform':
        case '-p':
          config.platform = args[++i] as Platform;
          break;
        case '--dev':
        case '-d':
          config.buildType = 'dev';
          break;
        case '--verbose':
        case '-v':
          config.verbose = true;
          break;
        case '--skip-frontend':
          config.skipFrontend = true;
          break;
        case '--targets':
        case '-t':
          config.targets = args[++i].split(',') as Target[];
          break;
        case '--help':
        case '-h':
          ManifestBuilder.showHelp();
          process.exit(0);
      }
    }

    return config;
  }

  static showHelp(): void {
    console.log(`
Manifest Build Script

Usage: npm run build:installers [options]

Options:
  -p, --platform <platform>    Target platform: windows, macos, linux, all (default: all)
  -d, --dev                    Development build (default: release)
  -v, --verbose                Verbose output
  -t, --targets <targets>      Comma-separated targets: msi,dmg,app,deb,appimage
  --skip-frontend              Skip frontend build
  -h, --help                   Show this help

Examples:
  npm run build:installers --platform windows --dev
  npm run build:installers --platform linux --targets deb
  npm run build:installers --verbose
    `);
  }
}

// Main execution
const isMainModule = import.meta.url === `file://${process.argv[1]}`;
if (isMainModule) {
  const config = ManifestBuilder.parseArgs();
  const builder = new ManifestBuilder(config);
  builder.build().catch(console.error);
}

export { ManifestBuilder, BuildConfig };
