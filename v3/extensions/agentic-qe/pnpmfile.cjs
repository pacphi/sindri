/**
 * pnpm hooks for agentic-qe dependency resolution
 *
 * Problem: agentic-qe@3.2.3 requires lodash@^4.17.23 but latest lodash is 4.17.21
 * Solution: Remap invalid dependency versions to valid ones
 *
 * This file should be merged into ~/.pnpmfile.cjs or used as global-pnpmfile
 */

const versionOverrides = {
  // agentic-qe@3.2.x requires lodash@^4.17.23 but latest is 4.17.21
  'lodash': {
    '^4.17.23': '^4.17.21',
    '^4.17.22': '^4.17.21'
  }
};

function readPackage(pkg, context) {
  for (const [depName, mappings] of Object.entries(versionOverrides)) {
    for (const depType of ['dependencies', 'devDependencies', 'optionalDependencies']) {
      if (pkg[depType] && pkg[depType][depName]) {
        const currentVersion = pkg[depType][depName];
        if (mappings[currentVersion]) {
          context.log(`[agentic-qe] Remapping ${pkg.name}'s ${depName}@${currentVersion} to ${mappings[currentVersion]}`);
          pkg[depType][depName] = mappings[currentVersion];
        }
      }
    }
  }
  return pkg;
}

module.exports = {
  hooks: {
    readPackage
  }
};
