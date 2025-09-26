/**
 * Camera Controller
 * Main orchestrator for camera modes and input handling
 */

import React, { useCallback, useEffect } from 'react';

import { useCameraStore } from '../../stores/camera-store';

import { CinematicCamera } from './components/CinematicCamera';
import { FreeCamera } from './components/FreeCamera';
import { OrbitalCamera } from './components/OrbitalCamera';
import { useCameraEffects } from './hooks/use-camera-effects';
import { useInputHandling } from './hooks/use-input-handling';
import type { CameraControlsProps } from './types';

/**
 * Main Camera Controller
 * Manages camera modes, effects, and input handling
 */
export const CameraController: React.FC<CameraControlsProps> = ({
  mode,
  constraints = {},
  enableShake = true,
  enableFocus = true,
  smoothTransitions = true,
  onModeChange,
}) => {
  const { currentMode, setMode, setConstraints } = useCameraStore();

  // Initialize input handling
  const inputHandling = useInputHandling({
    enableKeyboard: true,
    enableMouse: true,
    enableTouch: true,
    enableGamepad: false,
    edgeScrollThreshold: 50,
    scrollSpeed: 2,
  });

  // Initialize camera effects
  const _cameraEffects = useCameraEffects({
    enableShake,
    enableFocus,
    enableTransitions: smoothTransitions,
  });

  // Handle external mode changes
  useEffect(() => {
    if (mode && mode !== currentMode) {
      setMode(mode);
      onModeChange?.(mode);
    }
  }, [mode, currentMode, setMode, onModeChange]);

  // Apply constraints when they change
  useEffect(() => {
    if (Object.keys(constraints).length > 0) {
      setConstraints(currentMode, constraints);
    }
  }, [constraints, currentMode, setConstraints]);

  // Handle camera updates
  const handleCameraUpdate = useCallback((_camera: THREE.Camera) => {
    // Additional camera update logic can go here
    // This is called by individual camera components
  }, []);

  // Render appropriate camera component based on mode
  const renderCameraComponent = () => {
    switch (currentMode) {
      case 'orbital':
        return (
          <OrbitalCamera
            constraints={constraints}
            enableDamping
            dampingFactor={0.05}
            enableAutoRotate={false}
            onUpdate={handleCameraUpdate}
          />
        );

      case 'free':
        return (
          <FreeCamera
            movementSpeed={10}
            rollSpeed={0.3}
            dragToLook
            autoForward={false}
            onUpdate={handleCameraUpdate}
          />
        );

      case 'cinematic':
        return (
          <CinematicCamera
            enableFocusTracking={enableFocus}
            smoothness={0.1}
            lookAhead={2}
            onUpdate={handleCameraUpdate}
          />
        );

      case 'locked':
        return null; // No controls in locked mode

      default:
        return (
          <OrbitalCamera
            constraints={constraints}
            onUpdate={handleCameraUpdate}
          />
        );
    }
  };

  return (
    <>
      {/* Main camera component */}
      {renderCameraComponent()}

      {/* Input overlay for gesture handling */}
      {inputHandling.bind && (
        <div
          {...inputHandling.bind()}
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: '100%',
            pointerEvents: 'auto',
            touchAction: 'none',
          }}
        />
      )}

      {/* Context menu */}
      {inputHandling.contextMenu.visible && (
        <div
          style={{
            position: 'absolute',
            left: inputHandling.contextMenu.position.x,
            top: inputHandling.contextMenu.position.y,
            background: 'rgba(0, 0, 0, 0.8)',
            borderRadius: '8px',
            padding: '8px 0',
            minWidth: '150px',
            zIndex: 1000,
          }}
        >
          {inputHandling.contextMenu.items.map(item => (
            <button
              key={item.id}
              onClick={() => {
                item.action();
                inputHandling.hideContextMenu();
              }}
              disabled={item.disabled}
              style={{
                display: 'block',
                width: '100%',
                padding: '8px 16px',
                border: 'none',
                background: 'transparent',
                color: 'white',
                textAlign: 'left',
                cursor: 'pointer',
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}

      {/* Tooltip */}
      {inputHandling.tooltip.visible && (
        <div
          style={{
            position: 'absolute',
            left: inputHandling.tooltip.position.x,
            top: inputHandling.tooltip.position.y - 30,
            background: 'rgba(0, 0, 0, 0.8)',
            color: 'white',
            padding: '4px 8px',
            borderRadius: '4px',
            fontSize: '12px',
            pointerEvents: 'none',
            zIndex: 1001,
          }}
        >
          {inputHandling.tooltip.content}
        </div>
      )}

      {/* Selection box */}
      {inputHandling.selectionBox.active && (
        <div
          style={{
            position: 'absolute',
            left: Math.min(
              inputHandling.selectionBox.start.x,
              inputHandling.selectionBox.end.x
            ),
            top: Math.min(
              inputHandling.selectionBox.start.y,
              inputHandling.selectionBox.end.y
            ),
            width: Math.abs(
              inputHandling.selectionBox.end.x -
                inputHandling.selectionBox.start.x
            ),
            height: Math.abs(
              inputHandling.selectionBox.end.y -
                inputHandling.selectionBox.start.y
            ),
            border: '2px dashed rgba(255, 255, 255, 0.6)',
            background: 'rgba(255, 255, 255, 0.1)',
            pointerEvents: 'none',
            zIndex: 999,
          }}
        />
      )}
    </>
  );
};

export default CameraController;
