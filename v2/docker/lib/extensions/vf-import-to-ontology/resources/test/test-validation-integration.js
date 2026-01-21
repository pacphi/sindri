#!/usr/bin/env node

/**
 * Test OWL2 Validation Integration
 *
 * Verifies that the validation bridge correctly integrates with
 * the Python OWL2 validator.
 */

const fs = require('fs');
const path = require('path');
const { validateOntologyFile, validateContent } = require('../src/validation_bridge');

// Test data directory
const TEST_DATA_DIR = path.join(__dirname, 'data');

// Create test data if it doesn't exist
function setupTestData() {
  if (!fs.existsSync(TEST_DATA_DIR)) {
    fs.mkdirSync(TEST_DATA_DIR, { recursive: true });
  }

  // Valid OWL2 file
  const validFile = path.join(TEST_DATA_DIR, 'valid-ontology.md');
  fs.writeFileSync(validFile, `# Test Ontology - Machine Learning

## About

Machine learning is a subset of artificial intelligence.

## Ontology

\`\`\`clojure
# Class Declarations
Declaration(Class(ai:MachineLearning))
Declaration(Class(ai:DeepLearning))
Declaration(Class(ai:Algorithm))

# Hierarchy
SubClassOf(ai:DeepLearning ai:MachineLearning)
SubClassOf(ai:MachineLearning ai:ArtificialIntelligence)

# Properties
Declaration(ObjectProperty(ai:hasAlgorithm))
Domain(ai:hasAlgorithm ai:MachineLearning)
Range(ai:hasAlgorithm ai:Algorithm)

# Restrictions
SubClassOf(ai:DeepLearning ObjectSomeValuesFrom(ai:hasAlgorithm ai:NeuralNetwork))

# Annotations
AnnotationAssertion(rdfs:label ai:MachineLearning "Machine Learning")
AnnotationAssertion(rdfs:label ai:DeepLearning "Deep Learning")
\`\`\`

## Description

Additional description here.
`, 'utf-8');

  // Invalid OWL2 file (unbalanced parentheses)
  const invalidFile = path.join(TEST_DATA_DIR, 'invalid-ontology.md');
  fs.writeFileSync(invalidFile, `# Test Ontology - Invalid

## Ontology

\`\`\`clojure
# Missing closing parenthesis
Declaration(Class(ai:InvalidClass)

# Invalid namespace
Declaration(Class(xyz:UnknownNamespace))

# Unbalanced
SubClassOf(ai:ClassA ai:ClassB))
\`\`\`
`, 'utf-8');

  // File without ontology blocks
  const noOntologyFile = path.join(TEST_DATA_DIR, 'no-ontology.md');
  fs.writeFileSync(noOntologyFile, `# Regular Markdown File

Just some regular content without any ontology blocks.

## Section 1

More content here.
`, 'utf-8');

  return {
    validFile,
    invalidFile,
    noOntologyFile
  };
}

// Test functions
async function testValidFile(filePath) {
  console.log('\n📋 Test 1: Valid OWL2 File');
  console.log('=' .repeat(60));

  try {
    const result = await validateOntologyFile(filePath);

    console.log(`File: ${path.basename(filePath)}`);
    console.log(`Status: ${result.is_valid ? '✅ VALID' : '❌ INVALID'}`);
    console.log(`Total Axioms: ${result.total_axioms}`);
    console.log(`Errors: ${result.errors.length}`);
    console.log(`Warnings: ${result.warnings.length}`);

    if (result.is_valid) {
      console.log('\n✅ Test PASSED: File is valid as expected');
      return true;
    } else {
      console.log('\n❌ Test FAILED: Expected valid file');
      console.log('Errors:', result.errors);
      return false;
    }
  } catch (error) {
    console.log(`\n❌ Test FAILED: ${error.message}`);
    return false;
  }
}

async function testInvalidFile(filePath) {
  console.log('\n📋 Test 2: Invalid OWL2 File (Should Detect Errors)');
  console.log('=' .repeat(60));

  try {
    const result = await validateOntologyFile(filePath);

    console.log(`File: ${path.basename(filePath)}`);
    console.log(`Status: ${result.is_valid ? '✅ VALID' : '❌ INVALID'}`);
    console.log(`Total Axioms: ${result.total_axioms}`);
    console.log(`Errors: ${result.errors.length}`);
    console.log(`Warnings: ${result.warnings.length}`);

    if (!result.is_valid && result.errors.length > 0) {
      console.log('\nDetected Errors:');
      result.errors.slice(0, 3).forEach(err => {
        console.log(`  - Line ${err.line_number}: ${err.message}`);
      });
      console.log('\n✅ Test PASSED: Errors detected as expected');
      return true;
    } else {
      console.log('\n❌ Test FAILED: Expected errors but none found');
      return false;
    }
  } catch (error) {
    console.log(`\n❌ Test FAILED: ${error.message}`);
    return false;
  }
}

async function testNoOntologyFile(filePath) {
  console.log('\n📋 Test 3: File Without Ontology Blocks');
  console.log('=' .repeat(60));

  try {
    const result = await validateOntologyFile(filePath);

    console.log(`File: ${path.basename(filePath)}`);
    console.log(`Total Axioms: ${result.total_axioms}`);
    console.log(`Warnings: ${result.warnings.length}`);

    if (result.total_axioms === 0 && result.warnings.length > 0) {
      console.log('\n✅ Test PASSED: No ontology blocks detected');
      return true;
    } else {
      console.log('\n❌ Test FAILED: Expected warning about no ontology blocks');
      return false;
    }
  } catch (error) {
    console.log(`\n❌ Test FAILED: ${error.message}`);
    return false;
  }
}

async function testContentValidation() {
  console.log('\n📋 Test 4: In-Memory Content Validation');
  console.log('=' .repeat(60));

  const content = `# Test

\`\`\`clojure
Declaration(Class(ai:TestClass))
SubClassOf(ai:TestClass ai:ParentClass)
\`\`\`
`;

  try {
    const result = await validateContent(content);

    console.log(`Status: ${result.is_valid ? '✅ VALID' : '❌ INVALID'}`);
    console.log(`Total Axioms: ${result.total_axioms}`);

    if (result.total_axioms === 2) {
      console.log('\n✅ Test PASSED: Content validated successfully');
      return true;
    } else {
      console.log('\n❌ Test FAILED: Unexpected axiom count');
      return false;
    }
  } catch (error) {
    console.log(`\n❌ Test FAILED: ${error.message}`);
    return false;
  }
}

async function testNonExistentFile() {
  console.log('\n📋 Test 5: Non-Existent File (Error Handling)');
  console.log('=' .repeat(60));

  const nonExistentFile = path.join(TEST_DATA_DIR, 'does-not-exist.md');

  try {
    await validateOntologyFile(nonExistentFile);
    console.log('\n❌ Test FAILED: Expected error for non-existent file');
    return false;
  } catch (error) {
    console.log(`Error caught: ${error.message}`);
    console.log('\n✅ Test PASSED: Error handling works correctly');
    return true;
  }
}

// Main test runner
async function runTests() {
  console.log('\n🧪 OWL2 Validation Integration Test Suite');
  console.log('=' .repeat(60));

  // Setup
  console.log('\n📦 Setting up test data...');
  const testFiles = setupTestData();
  console.log(`   Created ${Object.keys(testFiles).length} test files`);

  // Run tests
  const results = [];

  results.push(await testValidFile(testFiles.validFile));
  results.push(await testInvalidFile(testFiles.invalidFile));
  results.push(await testNoOntologyFile(testFiles.noOntologyFile));
  results.push(await testContentValidation());
  results.push(await testNonExistentFile());

  // Summary
  console.log('\n' + '=' .repeat(60));
  console.log('📊 TEST SUMMARY');
  console.log('=' .repeat(60));

  const passed = results.filter(r => r === true).length;
  const failed = results.filter(r => r === false).length;

  console.log(`Total Tests: ${results.length}`);
  console.log(`✅ Passed: ${passed}`);
  console.log(`❌ Failed: ${failed}`);

  if (failed === 0) {
    console.log('\n🎉 All tests passed!');
    process.exit(0);
  } else {
    console.log('\n⚠️  Some tests failed. See details above.');
    process.exit(1);
  }
}

// Run if called directly
if (require.main === module) {
  runTests().catch(error => {
    console.error('Test suite error:', error);
    process.exit(1);
  });
}

module.exports = {
  runTests,
  testValidFile,
  testInvalidFile,
  testNoOntologyFile,
  testContentValidation,
  testNonExistentFile
};
