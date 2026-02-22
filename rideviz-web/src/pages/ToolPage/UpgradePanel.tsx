import { useEffect, useState } from 'react';
import { createCheckoutSession, verifyLicense } from '../../api/client';
import { captureEvent } from '../../analytics/posthog';

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

  useEffect(() => {
    captureEvent('rv_pro_panel_viewed', { has_pro_access: hasProAccess });
  }, [hasProAccess]);

  const handleCheckout = async () => {
    captureEvent('rv_checkout_started', {
      has_email: Boolean(email.trim()),
    });
    if (!email.trim()) {
      setStatus('Email is required.');
      captureEvent('rv_checkout_blocked', { reason: 'missing_email' });
      return;
    }
    setLoading(true);
    setStatus(null);
    try {
      const checkout = await createCheckoutSession(email.trim());
      window.open(checkout.checkout_url, '_blank', 'noopener,noreferrer');
      captureEvent('rv_checkout_opened', {
        mode: checkout.mode,
        email_domain: email.includes('@') ? email.split('@')[1] : null,
      });
      setStatus(checkout.mode === 'mock' ? 'Mock checkout opened.' : 'Checkout opened.');
    } catch (error) {
      captureEvent('rv_checkout_failed', {
        error: error instanceof Error ? error.message.slice(0, 160) : 'unknown_error',
      });
      setStatus(error instanceof Error ? error.message : 'Failed to start checkout.');
    } finally {
      setLoading(false);
    }
  };

  const handleVerify = async () => {
    const token = tokenInput.trim() || currentToken?.trim() || '';
    captureEvent('rv_license_verify_clicked', { has_token: Boolean(token) });
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
      captureEvent('rv_license_verify_succeeded', { pro: verification.pro });
      setStatus(verification.pro ? `Pro active for ${verification.email}` : 'Valid free token.');
    } catch (error) {
      onLicenseToken(null);
      captureEvent('rv_license_verify_failed', {
        error: error instanceof Error ? error.message.slice(0, 160) : 'unknown_error',
      });
      setStatus(error instanceof Error ? error.message : 'License verification failed.');
    } finally {
      setLoading(false);
    }
  };

  const handleSaveToken = () => {
    const token = tokenInput.trim();
    if (!token) {
      onLicenseToken(null);
      captureEvent('rv_license_token_cleared');
      setStatus('License token cleared.');
      return;
    }
    onLicenseToken(token);
    captureEvent('rv_license_token_saved');
    setStatus('License token saved locally.');
  };

  const toggleLicenseControls = () => {
    setShowLicenseControls((prev) => {
      const next = !prev;
      captureEvent('rv_license_controls_toggled', { open: next });
      if (next && !hasProAccess) {
        captureEvent('rv_pro_intent', { source: 'license_controls' });
      }
      return next;
    });
  };

  const handleClearToken = () => {
    onLicenseToken(null);
    captureEvent('rv_license_token_cleared');
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
        <button onClick={toggleLicenseControls} disabled={loading}>
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
              <button onClick={handleClearToken} disabled={loading}>
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
