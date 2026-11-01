// Independent glTF 2.0 verification using the official Khronos validator.
// node tools/validate-unreal-glb.cjs PATH_TO_VALIDATOR_PACKAGE KIT_DIRECTORY
const fs = require('fs');
const path = require('path');
async function main() {
  const [validatorPath, kitPath] = process.argv.slice(2);
  if (!validatorPath || !kitPath) throw new Error('Expected validator package path and kit directory');
  const validator = require(path.resolve(validatorPath));
  const root = path.resolve(kitPath);
  const manifest = JSON.parse(fs.readFileSync(path.join(root, 'Data/manifest.json'), 'utf8'));
  const reports = {};
  for (const asset of manifest.assets) {
    const file = path.join(root, asset.file);
    const report = await validator.validateBytes(new Uint8Array(fs.readFileSync(file)), {uri: asset.file, maxIssues: 100});
    reports[asset.name] = report;
    console.log(`${asset.name}: ${report.issues.numErrors} errors, ${report.issues.numWarnings} warnings, ${report.issues.numInfos} info, ${report.issues.numHints} hints`);
    if (report.issues.numErrors || report.issues.numWarnings) {
      console.error(JSON.stringify(report.issues, null, 2));
      throw new Error('GLB validation failed: ' + asset.name);
    }
  }
  fs.writeFileSync(path.join(root, 'Data/gltf-validation.json'), JSON.stringify(reports, null, 2));
}
main().catch(error => {console.error(error); process.exitCode = 1;});
