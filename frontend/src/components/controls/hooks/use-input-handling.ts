/**
 * Input Handling Hook
 * Manages keyboard, mouse, touch, and gamepad inputs for camera controls
 */

import { useGesture } from '@use-gesture/react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useHotkeys } from 'react-hotkeys-hook';
import * as THREE from 'three';

import { useCameraStore } from '../../../stores/camera-store';
import type {
  ContextMenuOptions,
  InputHandlerOptions,
  SelectionBox,
  TooltipData,
} from '../types';
import { CameraUtils } from '../utils/CameraUtils';

const DEFAULT_OPTIONS: InputHandlerOptions = {
  enableKeyboard: true,
  enableMouse: true,
  enableTouch: true,
  enableGamepad: false,
  edgeScrollThreshold: 50,
  scrollSpeed: 2,
};

export const useInputHandling = (
  options: Partial<InputHandlerOptions> = {}
) => {
  const finalOptions = useMemo(
    () => ({ ...DEFAULT_OPTIONS, ...options }),
    [options]
  );

  const { currentMode, setMode, addBookmark, bookmarks, goToBookmark } =
    useCameraStore();

  // Input state
  const [selectionBox, setSelectionBox] = useState<SelectionBox>({
    start: new THREE.Vector2(),
    end: new THREE.Vector2(),
    active: false,
  });

  const [contextMenu, setContextMenu] = useState<ContextMenuOptions>({
    position: new THREE.Vector2(),
    items: [],
    visible: false,
  });

  const [tooltip, setTooltip] = useState<TooltipData>({
    content: '',
    position: new THREE.Vector2(),
    visible: false,
  });

  const mousePosition = useRef(new THREE.Vector2());
  const isDragging = useRef(false);

  // Keyboard shortcuts
  useHotkeys('ctrl+1, cmd+1', () => setMode('orbital'), {
    enabled: finalOptions.enableKeyboard,
  });
  useHotkeys('ctrl+2, cmd+2', () => setMode('free'), {
    enabled: finalOptions.enableKeyboard,
  });
  useHotkeys('ctrl+3, cmd+3', () => setMode('cinematic'), {
    enabled: finalOptions.enableKeyboard,
  });
  useHotkeys('ctrl+0, cmd+0', () => setMode('locked'), {
    enabled: finalOptions.enableKeyboard,
  });

  useHotkeys(
    'ctrl+s, cmd+s',
    () => {
      // This would be handled by the camera system to create a bookmark
      const bookmark = CameraUtils.createBookmark(
        new THREE.PerspectiveCamera(), // Would get actual camera
        new THREE.Vector3(), // Would get actual target
        currentMode,
        `Bookmark ${bookmarks.length + 1}`
      );
      addBookmark(bookmark);
    },
    { enabled: finalOptions.enableKeyboard, preventDefault: true }
  );

  // Quick bookmark access
  useHotkeys(
    '1,2,3,4,5,6,7,8,9',
    event => {
      const index = Number.parseInt(event.key) - 1;
      if (index < bookmarks.length) {
        goToBookmark(bookmarks[index].id);
      }
    },
    { enabled: finalOptions.enableKeyboard }
  );

  // Edge scrolling
  const handleEdgeScroll = useCallback(
    (mousePos: THREE.Vector2) => {
      if (!finalOptions.enableMouse) return new THREE.Vector2();

      const screenSize = new THREE.Vector2(
        window.innerWidth,
        window.innerHeight
      );
      return CameraUtils.calculateEdgeScroll(
        mousePos,
        screenSize,
        finalOptions.edgeScrollThreshold,
        finalOptions.scrollSpeed
      );
    },
    [finalOptions]
  );

  // Context menu handlers
  const showContextMenu = useCallback(
    (position: THREE.Vector2, items: ContextMenuOptions['items']) => {
      setContextMenu({
        position,
        items,
        visible: true,
      });
    },
    []
  );

  const hideContextMenu = useCallback(() => {
    setContextMenu(prev => ({ ...prev, visible: false }));
  }, []);

  // Tooltip handlers
  const showTooltip = useCallback(
    (content: string, position: THREE.Vector2, delay = 500) => {
      setTimeout(() => {
        setTooltip({
          content,
          position,
          visible: true,
          delay,
        });
      }, delay);
    },
    []
  );

  const hideTooltip = useCallback(() => {
    setTooltip(prev => ({ ...prev, visible: false }));
  }, []);

  // Selection box handlers
  const startSelection = useCallback((start: THREE.Vector2) => {
    setSelectionBox({
      start,
      end: start.clone(),
      active: true,
    });
    isDragging.current = true;
  }, []);

  const updateSelection = useCallback(
    (end: THREE.Vector2) => {
      if (!selectionBox.active) return;

      setSelectionBox(prev => ({
        ...prev,
        end,
      }));
    },
    [selectionBox.active]
  );

  const endSelection = useCallback(() => {
    setSelectionBox(prev => ({
      ...prev,
      active: false,
    }));
    isDragging.current = false;
  }, []);

  // Gesture handlers using @use-gesture/react
  const bind = useGesture(
    {
      onDrag: ({ event, first, last }) => {
        if (!finalOptions.enableMouse && !finalOptions.enableTouch) return;

        const position = new THREE.Vector2(
          (event as MouseEvent).clientX,
          (event as MouseEvent).clientY
        );

        if (first) {
          startSelection(position);
        } else if (last) {
          endSelection();
        } else {
          updateSelection(position);
        }
      },

      onMove: ({ event }) => {
        const position = new THREE.Vector2(
          (event as MouseEvent).clientX,
          (event as MouseEvent).clientY
        );
        mousePosition.current = position;

        // Handle edge scrolling
        const scrollVector = handleEdgeScroll(position);
        if (scrollVector.length() > 0) {
          // Would emit edge scroll event here
        }
      },

      onContextMenu: ({ event }) => {
        if (!finalOptions.enableMouse) return;

        event.preventDefault();
        const position = new THREE.Vector2(
          (event as MouseEvent).clientX,
          (event as MouseEvent).clientY
        );

        const contextItems = [
          {
            id: 'focus',
            label: 'Focus Here',
            action: () => console.warn('Focus clicked'),
          },
          {
            id: 'bookmark',
            label: 'Save Bookmark',
            action: () => console.warn('Bookmark clicked'),
          },
          {
            id: 'mode-orbital',
            label: 'Orbital Mode',
            action: () => setMode('orbital'),
          },
          {
            id: 'mode-free',
            label: 'Free Mode',
            action: () => setMode('free'),
          },
        ];

        showContextMenu(position, contextItems);
      },

      onHover: ({ event, hovering }) => {
        const position = new THREE.Vector2(
          (event as MouseEvent).clientX,
          (event as MouseEvent).clientY
        );

        if (hovering) {
          // Would check what's under cursor and show appropriate tooltip
          showTooltip('Camera Controls Active', position);
        } else {
          hideTooltip();
        }
      },
    },
    {
      drag: {
        threshold: 5,
      },
    }
  );

  // Gamepad support (basic implementation)
  const updateGamepad = useCallback(() => {
    if (!finalOptions.enableGamepad || !navigator.getGamepads) return;

    const gamepads = navigator.getGamepads();
    const gamepad = gamepads[0];

    if (gamepad) {
      // Basic gamepad handling would go here
      // Left stick for movement, right stick for camera
      const _leftStick = { x: gamepad.axes[0], y: gamepad.axes[1] };
      const _rightStick = { x: gamepad.axes[2], y: gamepad.axes[3] };

      // Would emit gamepad movement events
    }
  }, [finalOptions.enableGamepad]);

  // Touch gesture support
  const handlePinch = useCallback((scale: number) => {
    // Would emit zoom event
    console.warn('Pinch zoom:', scale);
  }, []);

  const handleRotate = useCallback((rotation: number) => {
    // Would emit rotation event
    console.warn('Touch rotation:', rotation);
  }, []);

  return {
    // Gesture binding
    bind,

    // State
    selectionBox,
    contextMenu,
    tooltip,
    mousePosition: mousePosition.current,
    isDragging: isDragging.current,

    // Actions
    showContextMenu,
    hideContextMenu,
    showTooltip,
    hideTooltip,
    startSelection,
    updateSelection,
    endSelection,
    updateGamepad,
    handlePinch,
    handleRotate,

    // Edge scroll
    edgeScrollVector: handleEdgeScroll(mousePosition.current),
  };
};
