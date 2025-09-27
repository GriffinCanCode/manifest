/**
 * IPC Error Boundary Component
 * Provides graceful fallback when sophisticated IPC system fails
 */

import React from 'react';

import { createIPCErrorBoundary } from '@/utils/ipc';

const IPCErrorBoundary = createIPCErrorBoundary();

const FallbackComponent: React.FC<{
  error: Error;
  resetErrorBoundary: () => void;
}> = ({ error, resetErrorBoundary }) => {
  return (
    <div
      style={{
        padding: '2rem',
        background: 'rgba(255, 0, 0, 0.1)',
        border: '1px solid rgba(255, 0, 0, 0.3)',
        borderRadius: '8px',
        color: 'white',
        textAlign: 'center',
        maxWidth: '500px',
        margin: '2rem auto',
      }}
    >
      <h3 style={{ color: '#ff6b6b', marginBottom: '1rem' }}>
        🔧 IPC Communication Error
      </h3>
      <p style={{ marginBottom: '1rem', color: '#ccc' }}>
        There was an issue with the communication system. The app will continue
        using fallback methods.
      </p>
      <details style={{ marginBottom: '1rem', textAlign: 'left' }}>
        <summary style={{ cursor: 'pointer', color: '#ff6b6b' }}>
          Error Details (for developers)
        </summary>
        <pre
          style={{
            background: 'rgba(0, 0, 0, 0.3)',
            padding: '1rem',
            borderRadius: '4px',
            fontSize: '0.8rem',
            color: '#ddd',
            overflow: 'auto',
            marginTop: '0.5rem',
          }}
        >
          {error.message}
        </pre>
      </details>
      <button
        onClick={resetErrorBoundary}
        style={{
          background: '#3b82f6',
          color: 'white',
          border: 'none',
          padding: '0.5rem 1rem',
          borderRadius: '4px',
          cursor: 'pointer',
        }}
      >
        Continue
      </button>
    </div>
  );
};

/**
 * Wrapper component that provides IPC error boundary protection
 */
export const WithIPCErrorBoundary: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  return (
    <IPCErrorBoundary fallback={FallbackComponent}>{children}</IPCErrorBoundary>
  );
};

export default WithIPCErrorBoundary;
