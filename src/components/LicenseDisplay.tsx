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

interface CargoLicense {
  license: string;
  package: {
    name: string;
    version: string;
  };
  text: string;
}

const LicenseDisplay: React.FC<{ onBack: () => void }> = ({ onBack }) => {
  const [npmLicenses, setNpmLicenses] = useState<NpmLicense[]>([]);
  const [cargoLicenses, setCargoLicenses] = useState<CargoLicense[]>([]);
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

        // Fetch Cargo licenses
        const cargoRes = await fetch('/licenses-cargo.json');
        if (!cargoRes.ok) {
          throw new Error(`Failed to fetch cargo licenses: ${cargoRes.statusText}`);
        }
        const cargoData = await cargoRes.json();
        console.log("Full cargo data:", cargoData);
        if (cargoData.crates && cargoData.crates.length > 0) {
          console.log("First cargo crate object:", cargoData.crates[0]);
          console.log("Keys of first cargo crate:", Object.keys(cargoData.crates[0]));
        }
        setCargoLicenses(cargoData.crates || []);
        
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

      <h2>Frontend Dependencies</h2>
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

      <h2>Backend Dependencies</h2>
      {cargoLicenses.length > 0 ? (
        <div className="license-list">
          {cargoLicenses.map((license, index) => (
            <details key={`cargo-${index}`} className="license-item">
              <summary>
                {license.package.name}@{license.package.version} - ({license.license})
              </summary>
              <pre className="license-text">{license.text}</pre>
            </details>
          ))}
        </div>
      ) : (
        <p>No backend licenses found.</p>
      )}
    </div>
  );
};

export default LicenseDisplay; 
