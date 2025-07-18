import React, { useState, useEffect } from 'react';
import './LicenseDisplay.css';

interface NpmLicense {
  name: string;
  version: string;
  author?: string;
  repository?: string;
  source?: string;
  license?: string;
  licenseText?: string;
}

interface CargoLicense {
  name: string;
  version: string;
  license: string;
  license_file: string | null;
  text: string;
}

const LicenseDisplay: React.FC = () => {
  const [npmLicenses, setNpmLicenses] = useState<NpmLicense[]>([]);
  const [cargoLicenses, setCargoLicenses] = useState<CargoLicense[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchLicenses = async () => {
      try {
        setLoading(true);
        setError(null);
        console.log("Fetching licenses...");

        // Fetch NPM licenses
        console.log("Fetching /licenses.json...");
        const npmRes = await fetch('/licenses.json');
        console.log("NPM licenses response status:", npmRes.status);
        if (!npmRes.ok) {
          throw new Error(`Failed to fetch npm licenses: ${npmRes.statusText}`);
        }
        const npmData = await npmRes.json();
        console.log("NPM licenses data:", npmData);
        setNpmLicenses(npmData.dependencies || []);

        // Fetch Cargo licenses
        console.log("Fetching /licenses-cargo.json...");
        const cargoRes = await fetch('/licenses-cargo.json');
        console.log("Cargo licenses response status:", cargoRes.status);
        if (!cargoRes.ok) {
          throw new Error(`Failed to fetch cargo licenses: ${cargoRes.statusText}`);
        }
        const cargoData = await cargoRes.json();
        console.log("Cargo licenses data:", cargoData);
        setCargoLicenses(cargoData.licenses || []);

        console.log("Licenses fetched successfully.");
        
      } catch (e: any) {
        console.error("Error fetching licenses:", e);
        setError(`ライセンス情報の読み込みに失敗しました: ${e.message}`);
      } finally {
        setLoading(false);
      }
    };

    fetchLicenses();
  }, []);

  if (loading) {
    return <div>ライセンス情報を読み込んでいます...</div>;
  }

  if (error) {
    return <div>{error}</div>;
  }

  return (
    <div className="license-container">
      <h1>オープンソースライセンス</h1>

      <h2>Frontend Dependencies</h2>
      <div className="license-list">
        {npmLicenses.map((pkg) => (
          <details key={`${pkg.name}@${pkg.version}`} className="license-item">
            <summary>{pkg.name}@{pkg.version} - ({pkg.license})</summary>
            <pre className="license-text">{pkg.licenseText}</pre>
          </details>
        ))}
      </div>

      <h2>Backend Dependencies</h2>
      <div className="license-list">
        {cargoLicenses.map((pkg) => (
          <details key={`${pkg.name}@${pkg.version}`} className="license-item">
            <summary>{pkg.name}@{pkg.version} - ({pkg.license})</summary>
            <pre className="license-text">{pkg.text}</pre>
          </details>
        ))}
      </div>
    </div>
  );
};

export default LicenseDisplay; 
