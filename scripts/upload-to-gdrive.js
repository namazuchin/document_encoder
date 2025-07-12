#!/usr/bin/env node

import crypto from 'crypto';
import https from 'https';
import fs from 'fs';
import path from 'path';

// Command line arguments
const args = process.argv.slice(2);
if (args.length < 4) {
  console.error('Usage: node upload-to-gdrive.js <credentials_base64> <folder_id> <file_path> <upload_name>');
  process.exit(1);
}

const [credentialsBase64, folderId, filePath, uploadName] = args;

// Check folder ID
if (!folderId || folderId.trim() === '') {
  console.error('Error: GOOGLE_DRIVE_FOLDER_ID is empty or not set');
  console.error('Please ensure the GOOGLE_DRIVE_FOLDER_ID secret is properly configured in GitHub repository settings');
  process.exit(1);
}

// Decode and parse service account credentials
let credentials;
try {
  if (!credentialsBase64 || credentialsBase64.trim() === '') {
    console.error('Error: GOOGLE_DRIVE_CREDENTIALS is empty or not set');
    console.error('Please ensure the GOOGLE_DRIVE_CREDENTIALS secret is properly configured in GitHub repository settings');
    process.exit(1);
  }
  
  const credentialsJson = Buffer.from(credentialsBase64, 'base64').toString('utf8');
  credentials = JSON.parse(credentialsJson);
  
  // Validate required fields
  if (!credentials.private_key || !credentials.client_email) {
    console.error('Error: Invalid service account credentials - missing required fields');
    console.error('Required fields: private_key, client_email');
    process.exit(1);
  }
  
  console.log('Service account credentials loaded successfully');
  console.log('Client email:', credentials.client_email);
} catch (error) {
  console.error('Failed to parse credentials:', error.message);
  console.error('Please check that GOOGLE_DRIVE_CREDENTIALS contains a valid base64-encoded JSON file');
  process.exit(1);
}

// Check if file exists
console.log(`Checking file existence: ${filePath}`);
console.log(`Current working directory: ${process.cwd()}`);

// List files in current directory for debugging
try {
  const files = fs.readdirSync('.');
  console.log('Files in current directory:');
  files.forEach(file => {
    const stats = fs.statSync(file);
    console.log(`  ${file} (${stats.isDirectory() ? 'directory' : 'file'})`);
  });
} catch (error) {
  console.log('Could not list current directory:', error.message);
}

if (!fs.existsSync(filePath)) {
  // Try different path variations
  const fileName = path.basename(filePath);
  const possiblePaths = [
    filePath,
    fileName,
    `./${fileName}`,
    path.resolve(filePath),
    path.resolve(fileName)
  ];
  
  let foundPath = null;
  for (const testPath of possiblePaths) {
    if (fs.existsSync(testPath)) {
      foundPath = testPath;
      console.log(`Found file at: ${foundPath}`);
      break;
    }
  }
  
  if (!foundPath) {
    console.error(`File not found: ${filePath}`);
    console.error('Tried paths:');
    possiblePaths.forEach(p => console.error(`  - ${p}`));
    process.exit(1);
  } else {
    // Update filePath to the found path
    console.log(`Using found path: ${foundPath}`);
    const originalFilePath = filePath;
    const actualFilePath = foundPath;
    
    // Update the arguments for the rest of the script
    args[2] = actualFilePath;
  }
} else {
  console.log(`File exists: ${filePath}`);
}

// Create JWT for service account
function createJWT() {
  const header = {
    alg: 'RS256',
    typ: 'JWT'
  };

  const now = Math.floor(Date.now() / 1000);
  const payload = {
    iss: credentials.client_email,
    scope: 'https://www.googleapis.com/auth/drive',
    aud: 'https://oauth2.googleapis.com/token',
    exp: now + 3600,
    iat: now
  };

  const headerBase64 = Buffer.from(JSON.stringify(header)).toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  const payloadBase64 = Buffer.from(JSON.stringify(payload)).toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  
  const message = `${headerBase64}.${payloadBase64}`;
  
  // Sign with private key
  const signature = crypto.sign('RSA-SHA256', Buffer.from(message), credentials.private_key);
  const signatureBase64 = signature.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  
  return `${message}.${signatureBase64}`;
}

// Get access token using JWT
function getAccessToken() {
  return new Promise((resolve, reject) => {
    const jwt = createJWT();
    
    const postData = 'grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=' + encodeURIComponent(jwt);

    const options = {
      hostname: 'oauth2.googleapis.com',
      port: 443,
      path: '/token',
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Content-Length': Buffer.byteLength(postData)
      }
    };

    const req = https.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => {
        data += chunk;
      });
      res.on('end', () => {
        try {
          const response = JSON.parse(data);
          if (response.access_token) {
            resolve(response.access_token);
          } else {
            reject(new Error(`Failed to get access token: ${data}`));
          }
        } catch (error) {
          reject(new Error(`Failed to parse token response: ${error.message}`));
        }
      });
    });

    req.on('error', (error) => {
      reject(new Error(`Token request failed: ${error.message}`));
    });

    req.write(postData);
    req.end();
  });
}

// Check if file with same name exists in folder
function findExistingFile(accessToken, fileName) {
  return new Promise((resolve, reject) => {
    const query = `name='${fileName.replace(/'/g, "\\'")}' and '${folderId}' in parents and trashed=false`;
    const encodedQuery = encodeURIComponent(query);
    
    const options = {
      hostname: 'www.googleapis.com',
      port: 443,
      path: `/drive/v3/files?q=${encodedQuery}&fields=files(id,name)`,
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${accessToken}`
      }
    };

    const req = https.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => {
        data += chunk;
      });
      res.on('end', () => {
        try {
          const response = JSON.parse(data);
          const existingFile = response.files && response.files.length > 0 ? response.files[0] : null;
          resolve(existingFile);
        } catch (error) {
          reject(new Error(`Failed to parse search response: ${error.message}`));
        }
      });
    });

    req.on('error', (error) => {
      reject(new Error(`Search request failed: ${error.message}`));
    });

    req.end();
  });
}

// Upload file to Google Drive
function uploadFile(accessToken, existingFileId = null) {
  return new Promise((resolve, reject) => {
    // Use the potentially updated filePath from args
    const actualFilePath = args[2];
    const fileStats = fs.statSync(actualFilePath);
    const fileName = uploadName;
    const mimeType = getMimeType(path.extname(actualFilePath));

    console.log(`${existingFileId ? 'Updating' : 'Uploading'} file: ${fileName} (${fileStats.size} bytes)`);

    // Create metadata
    const metadata = existingFileId ? 
      { name: fileName } : 
      { name: fileName, parents: [folderId] };

    const boundary = '-------314159265358979323846';
    const delimiter = `\r\n--${boundary}\r\n`;
    const closeDelimiter = `\r\n--${boundary}--`;

    // Create multipart body
    const metadataPart = delimiter +
      'Content-Type: application/json\r\n\r\n' +
      JSON.stringify(metadata);
    
    const mediaPart = delimiter +
      `Content-Type: ${mimeType}\r\n\r\n`;

    const postDataStart = metadataPart + mediaPart;
    const postDataEnd = closeDelimiter;
    
    const contentLength = Buffer.byteLength(postDataStart) + fileStats.size + Buffer.byteLength(postDataEnd);

    const method = existingFileId ? 'PATCH' : 'POST';
    const url = existingFileId ? 
      `/upload/drive/v3/files/${existingFileId}?uploadType=multipart` :
      '/upload/drive/v3/files?uploadType=multipart';

    const options = {
      hostname: 'www.googleapis.com',
      port: 443,
      path: url,
      method: method,
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Content-Type': `multipart/related; boundary="${boundary}"`,
        'Content-Length': contentLength
      }
    };

    const req = https.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => {
        data += chunk;
      });
      res.on('end', () => {
        if (res.statusCode >= 200 && res.statusCode < 300) {
          try {
            const response = JSON.parse(data);
            console.log(`Successfully ${existingFileId ? 'updated' : 'uploaded'} file: ${response.name} (ID: ${response.id})`);
            resolve(response);
          } catch (error) {
            console.log(`${existingFileId ? 'Updated' : 'Uploaded'} successfully, but response parsing failed:`, error.message);
            resolve({ success: true });
          }
        } else {
          reject(new Error(`Upload failed with status ${res.statusCode}: ${data}`));
        }
      });
    });

    req.on('error', (error) => {
      reject(new Error(`Upload request failed: ${error.message}`));
    });

    // Write multipart data
    req.write(postDataStart);
    
    // Stream file data
    const fileStream = fs.createReadStream(actualFilePath);
    fileStream.on('data', (chunk) => {
      req.write(chunk);
    });
    
    fileStream.on('end', () => {
      req.write(postDataEnd);
      req.end();
    });

    fileStream.on('error', (error) => {
      reject(new Error(`File read error: ${error.message}`));
    });
  });
}

// Get MIME type based on file extension
function getMimeType(extension) {
  const mimeTypes = {
    '.dmg': 'application/x-apple-diskimage',
    '.msi': 'application/x-msi',
    '.zip': 'application/zip',
    '.tar.gz': 'application/gzip',
    '.exe': 'application/x-msdownload'
  };
  return mimeTypes[extension.toLowerCase()] || 'application/octet-stream';
}

// Main execution
async function main() {
  try {
    console.log('Getting access token...');
    const accessToken = await getAccessToken();
    console.log('Access token obtained successfully');

    console.log('Checking for existing files...');
    const existingFile = await findExistingFile(accessToken, uploadName);
    
    if (existingFile) {
      console.log(`Found existing file: ${existingFile.name} (ID: ${existingFile.id})`);
      await uploadFile(accessToken, existingFile.id);
    } else {
      console.log('No existing file found, creating new file');
      await uploadFile(accessToken);
    }
    
    console.log('Upload completed successfully!');
  } catch (error) {
    console.error('Upload failed:', error.message);
    process.exit(1);
  }
}

main();