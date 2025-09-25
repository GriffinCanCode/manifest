import React from 'react'

interface LoadingScreenProps {
  message?: string
}

const LoadingScreen: React.FC<LoadingScreenProps> = ({ 
  message = 'Loading Manifest...' 
}) => {
  return (
    <div className="loading-screen">
      <div className="loading-content">
        <div className="loading-logo">
          <h1>Manifest</h1>
        </div>
        <div className="loading-spinner">
          <div className="spinner"></div>
        </div>
        <p className="loading-message">{message}</p>
      </div>
      
      <style>{`
        .loading-screen {
          position: fixed;
          top: 0;
          left: 0;
          width: 100vw;
          height: 100vh;
          background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 9999;
          color: white;
          font-family: 'Inter', system-ui, sans-serif;
        }
        
        .loading-content {
          text-align: center;
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 2rem;
        }
        
        .loading-logo h1 {
          font-size: 4rem;
          font-weight: 700;
          background: linear-gradient(45deg, #64b5f6, #42a5f5, #2196f3);
          background-clip: text;
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
          margin: 0;
          letter-spacing: 0.1em;
        }
        
        .loading-spinner {
          width: 60px;
          height: 60px;
        }
        
        .spinner {
          width: 100%;
          height: 100%;
          border: 3px solid rgba(255, 255, 255, 0.1);
          border-top: 3px solid #2196f3;
          border-radius: 50%;
          animation: spin 1s linear infinite;
        }
        
        @keyframes spin {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }
        
        .loading-message {
          font-size: 1.2rem;
          color: rgba(255, 255, 255, 0.8);
          margin: 0;
          font-weight: 300;
        }
      `}</style>
    </div>
  )
}

export default LoadingScreen
