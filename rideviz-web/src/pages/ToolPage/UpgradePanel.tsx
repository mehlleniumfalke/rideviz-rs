import { useEffect, useState } from 'react';
import { createCheckoutSession, verifyLicense } from '../../api/client';

interface UpgradePanelProps {
  onLicenseToken: (token: string | null) => void;
  currentToken: string | null;
  hasProAccess: boolean;
}

export default function UpgradePanel({ onLicenseToken, currentToken, hasProAccess }: UpgradePanelProps) {
  const [email, setEmail] = useState('');
  const [tokenInput, setTokenInput] = useState(currentToken ?? '');
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [showLicenseControls, setShowLicenseControls] = useState(false);

  useEffect(() => {
    setTokenInput(currentToken ?? '');
  }, [currentToken]);

  const handleCheckout = async () => {
    if (!email.trim()) {
      setStatus('Email is required.');
      return;
    }
    setLoading(true);
    setStatus(null);
    try {
      const checkout = await createCheckoutSession(email.trim());
      window.open(checkout.checkout_url, '_blank', 'noopener,noreferrer');
      setStatus(checkout.mode === 'mock' ? 'Mock checkout opened.' : 'Checkout opened.');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : 'Failed to start checkout.');
    } finally {
      setLoading(false);
    }
  };

  const handleVerify = async () => {
    const token = tokenInput.trim() || currentToken?.trim() || '';
    if (!token) {
      setStatus('No license token set.');
      return;
    }
    if (token !== currentToken) {
      onLicenseToken(token);
    }
    setLoading(true);
    setStatus(null);
    try {
      const verification = await verifyLicense(token);
      setStatus(verification.pro ? `Pro active for ${verification.email}` : 'Valid free token.');
    } catch (error) {
      onLicenseToken(null);
      setStatus(error instanceof Error ? error.message : 'License verification failed.');
    } finally {
      setLoading(false);
    }
  };

  const handleSaveToken = () => {
    const token = tokenInput.trim();
    if (!token) {
      onLicenseToken(null);
      setStatus('License token cleared.');
      return;
    }
    onLicenseToken(token);
    setStatus('License token saved locally.');
  };

  return (
    <div className="box-inverted">
      <div className="label">Upgrade to Pro</div>
      <div style={{ display: 'grid', gap: 'var(--space-2)' }}>
        {hasProAccess && (
          <div
            style={{
              border: '1px solid #333',
              padding: 'var(--space-2)',
              fontSize: 'var(--text-xs)',
              color: '#4ade80',
            }}
            aria-live="polite"
          >
            Pro active. Valid license key is configured on this device.
          </div>
        )}
        {!hasProAccess && (
          <>
            <input
              type="email"
              placeholder="email for checkout"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
            />
            <button onClick={handleCheckout} disabled={loading}>
              {loading ? 'Working...' : 'Start Checkout'}
            </button>
          </>
        )}
        <button onClick={() => setShowLicenseControls((prev) => !prev)} disabled={loading}>
          {showLicenseControls ? 'Hide License Key' : hasProAccess ? 'Manage License Key' : 'Enter License Key'}
        </button>
        {showLicenseControls && (
          <>
            <input
              type="text"
              placeholder="paste license token"
              value={tokenInput}
              onChange={(event) => setTokenInput(event.target.value)}
            />
            <button onClick={handleSaveToken} disabled={loading}>
              Save License Token
            </button>
            <button onClick={handleVerify} disabled={loading}>
              Verify License
            </button>
            {currentToken && (
              <button onClick={() => onLicenseToken(null)} disabled={loading}>
                Clear License
              </button>
            )}
          </>
        )}
        {status && (
          <div style={{ fontSize: 'var(--text-xs)', color: '#888' }} aria-live="polite">
            {status}
          </div>
        )}
      </div>
    </div>
  );
}
