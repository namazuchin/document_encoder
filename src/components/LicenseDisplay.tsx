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
        // vite build後は dist/licenses.json に配置されるが、開発中は public/licenses.json を参照する
        const npmRes = await fetch('/licenses.json');
        const npmData = await npmRes.json();
        setNpmLicenses(npmData.dependencies);

        // cargo-aboutで生成したファイルは src-tauri にあるので、TauriのファイルシステムAPI経由で読む必要がある
        // しかし、セキュリティ上の理由から直接fs APIを使うのは難しいため、
        // ここではビルドプロセスで public ディレクトリにコピーされることを想定し、
        // fetchで取得する。
        // `tauri.conf.json` の `resources` に `"src-tauri/licenses-cargo.json"` を追加する必要がある。
        const cargoRes = await fetch('/licenses-cargo.json');
        const cargoData = await cargoRes.json();
        setCargoLicenses(cargoData.licenses);
        
      } catch (e) {
        console.error(e);
        setError('ライセンス情報の読み込みに失敗しました。');
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
