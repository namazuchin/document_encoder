import React, { useState, useEffect } from 'react';
import './LicenseDisplay.css';

interface NpmLicense {
  name: string;
  version: string;
  author?: string | { name: string };
  repository?: string | { url: string };
  license?: string;
  licenseText?: string;
}


const LicenseDisplay: React.FC<{ onBack: () => void }> = ({ onBack }) => {
  const [npmLicenses, setNpmLicenses] = useState<NpmLicense[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchLicenses = async () => {
      try {
        setLoading(true);
        setError(null);

        // Fetch NPM licenses
        const npmRes = await fetch('/licenses.json');
        if (!npmRes.ok) {
          throw new Error(`Failed to fetch npm licenses: ${npmRes.statusText}`);
        }
        const npmData = await npmRes.json();
        setNpmLicenses(npmData.dependencies || []);
        
      } catch (e: any) {
        setError(`ライセンス情報の読み込みに失敗しました: ${e.message}`);
      } finally {
        setLoading(false);
      }
    };

    fetchLicenses();
  }, []);

  if (loading) {
    return <div>Loading licenses...</div>;
  }

  if (error) {
    return <div>{error}</div>;
  }

  return (
    <div className="license-container">
      <button onClick={onBack} className="back-button">← 設定に戻る</button>
      <h1>オープンソースライセンス</h1>

      <div className="license-list">
        {npmLicenses.map((license) => (
          <details key={`${license.name}@${license.version}`} className="license-item">
            <summary>
              {license.name}@{license.version} - ({license.license})
            </summary>
            <pre className="license-text">{license.licenseText}</pre>
          </details>
        ))}
      </div>
    </div>
  );
};

export default LicenseDisplay; 
