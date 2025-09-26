/**
 * Render pass exports and utilities
 * Centralized pass management for multi-step rendering
 */

import {
  GeometryPass,
  PostProcessPass,
  ShadowPass,
  type RenderPass,
} from '../core/RenderPass';

export {
  RenderPipeline,
  useRenderPipeline,
} from '../components/pipeline/RenderPipeline';
export {
  GeometryPass,
  PostProcessPass,
  RenderPass,
  ShadowPass,
} from '../core/RenderPass';

// Advanced rendering passes
export { DebugPass } from './DebugPass';
export { DepthOfFieldPass } from './DepthOfFieldPass';
export { SelectionPass } from './SelectionPass';
export { VolumetricFogPass } from './VolumetricFogPass';

// Culling passes

// Pass type registry for extensibility
export type PassType =
  | 'geometry'
  | 'shadow'
  | 'postprocess'
  | 'ui'
  | 'debug'
  | 'dof'
  | 'fog'
  | 'selection'
  | 'volumetric-fog'
  | 'custom';

export interface PassRegistration {
  type: PassType;
  factory: () => RenderPass;
  priority: number;
  dependencies?: PassType[];
}

/**
 * Enhanced pass registry with dependency management and validation
 */
class PassRegistry {
  private passes = new Map<string, PassRegistration>();
  private dependencyGraph = new Map<string, string[]>();

  register(name: string, registration: PassRegistration): void {
    // Validate registration
    if (this.passes.has(name)) {
      console.warn(`Pass '${name}' is already registered, overwriting...`);
    }

    // Validate dependencies exist
    if (registration.dependencies) {
      for (const dep of registration.dependencies) {
        if (!this.hasPassOfType(dep)) {
          console.warn(`Dependency '${dep}' for pass '${name}' not found`);
        }
      }
    }

    this.passes.set(name, registration);
    this.dependencyGraph.set(name, registration.dependencies ?? []);
  }

  unregister(name: string): void {
    // Check if other passes depend on this one
    const dependents = this.getDependents(name);
    if (dependents.length > 0) {
      console.warn(
        `Pass '${name}' has dependents: ${dependents.join(', ')}. ` +
          'Consider unregistering dependents first.'
      );
    }

    this.passes.delete(name);
    this.dependencyGraph.delete(name);
  }

  get(name: string): PassRegistration | undefined {
    return this.passes.get(name);
  }

  getAll(): Map<string, PassRegistration> {
    return new Map(this.passes);
  }

  /**
   * Check if a pass of the given type exists
   */
  hasPassOfType(type: PassType): boolean {
    return Array.from(this.passes.values()).some(p => p.type === type);
  }

  /**
   * Get all passes that depend on the given pass type
   */
  getDependents(passName: string): string[] {
    const targetType = this.passes.get(passName)?.type;
    if (!targetType) return [];

    return Array.from(this.passes.entries())
      .filter(([_, reg]) => reg.dependencies?.includes(targetType))
      .map(([name]) => name);
  }

  /**
   * Validate the dependency graph for cycles
   */
  validateDependencies(): { valid: boolean; cycles: string[][] } {
    const visited = new Set<string>();
    const recursionStack = new Set<string>();
    const cycles: string[][] = [];

    const findCycles = (passName: string, path: string[]): boolean => {
      if (recursionStack.has(passName)) {
        // Found a cycle - extract it from the path
        const cycleStart = path.indexOf(passName);
        cycles.push(path.slice(cycleStart).concat(passName));
        return true;
      }

      if (visited.has(passName)) {
        return false;
      }

      visited.add(passName);
      recursionStack.add(passName);

      const passReg = this.passes.get(passName);
      if (passReg?.dependencies) {
        for (const depType of passReg.dependencies) {
          // Find pass with this type
          const depPass = Array.from(this.passes.entries()).find(
            ([_, reg]) => reg.type === depType
          );

          if (depPass && findCycles(depPass[0], [...path, passName])) {
            recursionStack.delete(passName);
            return true;
          }
        }
      }

      recursionStack.delete(passName);
      return false;
    };

    for (const passName of this.passes.keys()) {
      if (!visited.has(passName)) {
        findCycles(passName, []);
      }
    }

    return { valid: cycles.length === 0, cycles };
  }

  /**
   * Create passes in dependency order using topological sort
   */
  createOrderedPasses(): RenderPass[] {
    // Validate dependencies first
    const validation = this.validateDependencies();
    if (!validation.valid) {
      console.error('Circular dependencies detected:', validation.cycles);
      // Fall back to priority-based ordering
      return this.createOrderedPassesByPriority();
    }

    const result: RenderPass[] = [];
    const visited = new Set<string>();
    const visiting = new Set<string>();

    const visit = (passName: string): void => {
      if (visiting.has(passName)) {
        console.warn(`Circular dependency detected at '${passName}'`);
        return;
      }

      if (visited.has(passName)) {
        return;
      }

      visiting.add(passName);

      const passReg = this.passes.get(passName);
      if (!passReg) {
        visiting.delete(passName);
        return;
      }

      // Visit dependencies first
      if (passReg.dependencies) {
        for (const depType of passReg.dependencies) {
          // Find pass with this type
          const depPass = Array.from(this.passes.entries()).find(
            ([_, reg]) => reg.type === depType
          );

          if (depPass) {
            visit(depPass[0]);
          }
        }
      }

      visiting.delete(passName);
      visited.add(passName);

      // Create pass instance
      try {
        const pass = passReg.factory();
        result.push(pass);
      } catch (error) {
        console.error(`Failed to create pass '${passName}':`, error);
      }
    };

    // Visit all passes
    for (const passName of this.passes.keys()) {
      visit(passName);
    }

    return result;
  }

  /**
   * Fallback: Create passes sorted by priority only
   */
  private createOrderedPassesByPriority(): RenderPass[] {
    const registrations = Array.from(this.passes.values());
    registrations.sort((a, b) => a.priority - b.priority);

    return registrations
      .map(reg => {
        try {
          return reg.factory();
        } catch (error) {
          console.error(`Failed to create pass:`, error);
          return null;
        }
      })
      .filter(Boolean) as RenderPass[];
  }

  /**
   * Get detailed registry information for debugging
   */
  getDebugInfo(): {
    passes: Array<{
      name: string;
      type: PassType;
      priority: number;
      dependencies: PassType[];
    }>;
    validation: { valid: boolean; cycles: string[][] };
  } {
    const passes = Array.from(this.passes.entries()).map(([name, reg]) => ({
      name,
      type: reg.type,
      priority: reg.priority,
      dependencies: reg.dependencies ?? [],
    }));

    return {
      passes,
      validation: this.validateDependencies(),
    };
  }
}

export const passRegistry = new PassRegistry();

/**
 * Debug utilities for pass registry
 */
export const debugPassRegistry = () => {
  if (__DEV__) {
    const info = passRegistry.getDebugInfo();
    console.warn('🎨 Pass Registry Debug Info:', info.passes);

    if (!info.validation.valid) {
      console.error(
        '❌ Circular dependencies detected:',
        info.validation.cycles
      );
    } else {
      console.warn('✅ All dependencies valid');
    }

    return info;
  }
  return null;
};

// Register default passes
passRegistry.register('shadow', {
  type: 'shadow',
  factory: () => new ShadowPass(),
  priority: -10,
});

passRegistry.register('geometry', {
  type: 'geometry',
  factory: () => new GeometryPass(),
  priority: 0,
  dependencies: ['shadow'],
});

// Register advanced rendering passes
import { DebugPass } from './DebugPass';
import { DepthOfFieldPass } from './DepthOfFieldPass';
import { SelectionPass } from './SelectionPass';
import { VolumetricFogPass } from './VolumetricFogPass';

passRegistry.register('volumetric-fog', {
  type: 'volumetric-fog',
  factory: () => new VolumetricFogPass(),
  priority: 85,
  dependencies: ['geometry'],
});

passRegistry.register('depth-of-field', {
  type: 'dof',
  factory: () => new DepthOfFieldPass(),
  priority: 90,
  dependencies: ['geometry'],
});

passRegistry.register('selection', {
  type: 'selection',
  factory: () => new SelectionPass(),
  priority: 95,
  dependencies: ['geometry'],
});

passRegistry.register('debug', {
  type: 'debug',
  factory: () => new DebugPass(),
  priority: 200,
  dependencies: [],
});

passRegistry.register('postprocess', {
  type: 'postprocess',
  factory: () => new PostProcessPass(),
  priority: 100,
  dependencies: ['geometry'],
});
