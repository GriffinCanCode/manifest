/**
 * Camera and Scene Exposure Diagnostics
 * Helps identify camera positioning and scene visibility issues
 */

export function exposeCameraAndScene() {
  if (typeof window === 'undefined') return;

  // Hook into React Three Fiber to expose camera and scene
  const originalRAF = window.requestAnimationFrame;

  window.requestAnimationFrame = function (callback) {
    return originalRAF.call(this, function (time) {
      // Try to find and expose Three.js objects
      const canvas = document.querySelector('canvas');
      if (canvas) {
        // Try to get the Three.js context from the canvas
        const contexts = ['webgl2', 'webgl'];
        for (const contextType of contexts) {
          const gl = canvas.getContext(contextType);
          if (gl && (gl as any).__THREE_RENDERER__) {
            const renderer = (gl as any).__THREE_RENDERER__;
            if (renderer.info && renderer.info.render.calls > 0) {
              // Renderer is active, try to expose scene and camera
              const win = window as any;
              if (!win.__scene && renderer.scene) {
                win.__scene = renderer.scene;
                console.log('🎨 Exposed Three.js scene to window.__scene');
              }
              if (!win.__camera && renderer.camera) {
                win.__camera = renderer.camera;
                console.log('📷 Exposed Three.js camera to window.__camera');
              }
            }
          }
        }
      }

      callback(time);
    });
  };

  // Also try direct DOM inspection for React Three Fiber
  setTimeout(() => {
    inspectReactThreeFiber();
  }, 2000);
}

function inspectReactThreeFiber() {
  const canvas = document.querySelector('canvas');
  if (!canvas) return;

  // React Three Fiber stores the state on the canvas
  const fiberState = (canvas as any).__r3f;
  if (fiberState) {
    const win = window as any;

    if (fiberState.scene && !win.__scene) {
      win.__scene = fiberState.scene;
      console.log('🎨 Exposed R3F scene to window.__scene');
    }

    if (fiberState.camera && !win.__camera) {
      win.__camera = fiberState.camera;
      console.log('📷 Exposed R3F camera to window.__camera');
    }

    // Also expose the full R3F state for debugging
    win.__r3f = fiberState;
    console.log('⚛️ Exposed full R3F state to window.__r3f');
  }
}

// Auto-expose on import
exposeCameraAndScene();
