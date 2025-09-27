/**
 * IPC Status Component
 * Shows the status of the IPC communication system
 */

import React, { useEffect, useState } from 'react';

import { useIPCStatus } from '@/hooks/use-ipc';
import { getGlobalIPC } from '@/utils/ipc';

interface IPCStatusProps {
  showDetails?: boolean;
  compact?: boolean;
}

export const IPCStatus: React.FC<IPCStatusProps> = ({
  showDetails = false,
  compact = true,
}) => {
  const [isIPCAvailable, setIsIPCAvailable] = useState(false);
  const [lastCommandTime, setLastCommandTime] = useState<number | null>(null);
  const [commandCount, setCommandCount] = useState(0);

  const { getIPCHistory, getIPCMetrics } = useIPCStatus();

  // Check IPC availability
  useEffect(() => {
    try {
      const ipc = getGlobalIPC();
      setIsIPCAvailable(!!ipc);

      // Get basic metrics
      const metrics = getIPCMetrics();
      setCommandCount(metrics.overall.totalCommands || 0);

      const history = getIPCHistory();
      if (history.length > 0) {
        setLastCommandTime(history[history.length - 1]?.timestamp || null);
      }
    } catch (error) {
      setIsIPCAvailable(false);
    }
  }, [getIPCMetrics, getIPCHistory]);

  if (compact) {
    return (
      <div
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '0.5rem',
          padding: '0.25rem 0.5rem',
          background: isIPCAvailable
            ? 'rgba(34, 197, 94, 0.1)'
            : 'rgba(239, 68, 68, 0.1)',
          border: `1px solid ${
            isIPCAvailable ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)'
          }`,
          borderRadius: '4px',
          fontSize: '0.75rem',
          color: isIPCAvailable ? '#22c55e' : '#ef4444',
        }}
      >
        <div
          style={{
            width: '6px',
            height: '6px',
            borderRadius: '50%',
            background: isIPCAvailable ? '#22c55e' : '#ef4444',
          }}
        />
        <span>{isIPCAvailable ? 'IPC Active' : 'IPC Fallback'}</span>
        {showDetails && commandCount > 0 && (
          <span style={{ opacity: 0.7 }}>({commandCount} commands)</span>
        )}
      </div>
    );
  }

  return (
    <div
      style={{
        padding: '1rem',
        background: 'rgba(0, 0, 0, 0.2)',
        borderRadius: '8px',
        border: '1px solid rgba(255, 255, 255, 0.1)',
        color: 'white',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '0.5rem',
          marginBottom: '0.5rem',
        }}
      >
        <div
          style={{
            width: '8px',
            height: '8px',
            borderRadius: '50%',
            background: isIPCAvailable ? '#22c55e' : '#ef4444',
          }}
        />
        <strong>
          {isIPCAvailable
            ? '✅ Sophisticated IPC System Active'
            : '⚠️ Using Fallback IPC'}
        </strong>
      </div>

      {showDetails && (
        <div style={{ fontSize: '0.85rem', color: '#ccc' }}>
          <div>Commands executed: {commandCount}</div>
          {lastCommandTime && (
            <div>
              Last command: {new Date(lastCommandTime).toLocaleTimeString()}
            </div>
          )}
          <div style={{ marginTop: '0.5rem', fontSize: '0.75rem' }}>
            {isIPCAvailable
              ? 'Full validation, error handling, and progress tracking enabled'
              : 'Using direct invoke calls with basic error handling'}
          </div>
        </div>
      )}
    </div>
  );
};

export default IPCStatus;
