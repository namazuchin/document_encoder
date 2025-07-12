#!/usr/bin/env node

import crypto from 'crypto';
import https from 'https';
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';

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
function listDirectoryRecursive(dir, depth = 0, maxDepth = 2) {
  if (depth > maxDepth) return;
  
  try {
    const files = fs.readdirSync(dir);
    files.forEach(file => {
      const filePath = path.join(dir, file);
      const stats = fs.statSync(filePath);
      const indent = '  '.repeat(depth);
      console.log(`${indent}${file} (${stats.isDirectory() ? 'directory' : 'file'})`);
      
      if (stats.isDirectory()) {
        listDirectoryRecursive(filePath, depth + 1, maxDepth);
      }
    });
  } catch (error) {
    console.log(`Could not list directory ${dir}:`, error.message);
  }
}

console.log('Files in current directory (recursive):');
listDirectoryRecursive('.');

// Extract zip files if found
function extractZipFiles() {
  const zipFiles = [];
  
  function findZips(dir, depth = 0, maxDepth = 2) {
    if (depth > maxDepth) return;
    
    try {
      const files = fs.readdirSync(dir);
      for (const file of files) {
        const fullPath = path.join(dir, file);
        const stats = fs.statSync(fullPath);
        
        if (stats.isFile() && path.extname(file).toLowerCase() === '.zip') {
          zipFiles.push(fullPath);
        } else if (stats.isDirectory()) {
          findZips(fullPath, depth + 1, maxDepth);
        }
      }
    } catch (error) {
      console.log(`Could not search directory ${dir}:`, error.message);
    }
  }
  
  findZips('.');
  
  for (const zipFile of zipFiles) {
    console.log(`Found zip file: ${zipFile}`);
    try {
      const extractDir = path.dirname(zipFile);
      console.log(`Extracting ${zipFile} to ${extractDir}`);
      execSync(`unzip -o "${zipFile}" -d "${extractDir}"`, { stdio: 'inherit' });
      console.log(`Successfully extracted ${zipFile}`);
    } catch (error) {
      console.log(`Failed to extract ${zipFile}:`, error.message);
    }
  }
}

// Search for files matching the expected extensions
function findBuildArtifact() {
  const extensions = ['.dmg', '.msi', '.exe'];
  const searchDirs = ['.', './artifacts', './build', './dist', './target'];
  
  for (const dir of searchDirs) {
    if (fs.existsSync(dir)) {
      try {
        const files = fs.readdirSync(dir);
        for (const file of files) {
          const fullPath = path.join(dir, file);
          const ext = path.extname(file).toLowerCase();
          if (extensions.includes(ext)) {
            console.log(`Found potential artifact: ${fullPath}`);
            return fullPath;
          }
        }
      } catch (error) {
        console.log(`Could not search directory ${dir}:`, error.message);
      }
    }
  }
  
  // Recursive search for artifact files
  function searchRecursive(dir, depth = 0, maxDepth = 3) {
    if (depth > maxDepth) return null;
    
    try {
      const files = fs.readdirSync(dir);
      
      // First pass: look for files
      for (const file of files) {
        const fullPath = path.join(dir, file);
        const stats = fs.statSync(fullPath);
        
        if (stats.isFile()) {
          const ext = path.extname(file).toLowerCase();
          if (extensions.includes(ext)) {
            console.log(`Found artifact in recursive search: ${fullPath}`);
            return fullPath;
          }
        }
      }
      
      // Second pass: search subdirectories
      for (const file of files) {
        const fullPath = path.join(dir, file);
        const stats = fs.statSync(fullPath);
        
        if (stats.isDirectory()) {
          const found = searchRecursive(fullPath, depth + 1, maxDepth);
          if (found) return found;
        }
      }
    } catch (error) {
      console.log(`Could not recursively search directory ${dir}:`, error.message);
    }
    
    return null;
  }
  
  return searchRecursive('.');
}

if (!fs.existsSync(filePath)) {
  console.log('Original path not found, searching for build artifacts...');
  
  // First, extract any zip files that might contain artifacts
  console.log('Extracting zip files...');
  extractZipFiles();
  
  // Refresh directory listing after extraction
  console.log('Files after extraction:');
  listDirectoryRecursive('.');
  
  const foundArtifact = findBuildArtifact();
  
  if (foundArtifact) {
    console.log(`Using found artifact: ${foundArtifact}`);
    args[2] = foundArtifact;
  } else {
    console.error(`No build artifacts found. Expected file: ${filePath}`);
    console.error('Searched for files with extensions: .dmg, .msi, .exe');
    process.exit(1);
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
      path: `/drive/v3/files?q=${encodedQuery}&fields=files(id,name)&supportsAllDrives=true&includeItemsFromAllDrives=true`,
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
      `/upload/drive/v3/files/${existingFileId}?uploadType=multipart&supportsAllDrives=true` :
      '/upload/drive/v3/files?uploadType=multipart&supportsAllDrives=true';

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

// Verify folder exists and is accessible
function verifyFolder(accessToken) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'www.googleapis.com',
      port: 443,
      path: `/drive/v3/files/${folderId}?fields=id,name,parents,driveId&supportsAllDrives=true`,
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
        if (res.statusCode >= 200 && res.statusCode < 300) {
          try {
            const response = JSON.parse(data);
            console.log('Folder verification successful:');
            console.log(`  Name: ${response.name}`);
            console.log(`  ID: ${response.id}`);
            console.log(`  Drive ID: ${response.driveId || 'Personal Drive'}`);
            console.log(`  Parents: ${response.parents ? response.parents.join(', ') : 'Root'}`);
            
            if (response.driveId) {
              console.log('✓ Using Shared Drive (recommended)');
            } else {
              console.log('⚠ Warning: Using Personal Drive (may cause quota issues)');
            }
            
            resolve(response);
          } catch (error) {
            reject(new Error(`Failed to parse folder info: ${error.message}`));
          }
        } else {
          reject(new Error(`Folder verification failed with status ${res.statusCode}: ${data}`));
        }
      });
    });

    req.on('error', (error) => {
      reject(new Error(`Folder verification request failed: ${error.message}`));
    });

    req.end();
  });
}

// Main execution
async function main() {
  try {
    console.log('Getting access token...');
    const accessToken = await getAccessToken();
    console.log('Access token obtained successfully');

    console.log('Verifying folder access...');
    await verifyFolder(accessToken);

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
    
    // Provide helpful suggestions based on error type
    if (error.message.includes('404') || error.message.includes('notFound')) {
      console.error('\n📋 Troubleshooting suggestions:');
      console.error('1. Check that GOOGLE_DRIVE_FOLDER_ID is correct');
      console.error('2. Ensure the folder is in a Shared Drive (not personal Drive)');
      console.error('3. Verify the Service Account has been added to the Shared Drive with Editor permissions');
      console.error('4. Make sure the folder ID is from the correct Google account');
    } else if (error.message.includes('403')) {
      console.error('\n📋 Troubleshooting suggestions:');
      console.error('1. Use a Shared Drive instead of personal Drive');
      console.error('2. Add the Service Account as a member with Editor permissions');
      console.error('3. Ensure Google Drive API is enabled in your Google Cloud project');
    }
    
    process.exit(1);
  }
}

main();