#!/usr/bin/env node
/**
 * DeepSeek Chat CLI
 * Direct interface to DeepSeek API for deepseek-user
 *
 * Usage:
 *   deepseek-chat "your prompt here"
 *   deepseek-chat --interactive
 */

const https = require('https');
const readline = require('readline');

const API_KEY = process.env.DEEPSEEK_API_KEY || 'sk-[your deepseek api key]';
const API_URL = 'api.deepseek.com';
const MODEL = process.env.DEEPSEEK_MODEL || 'deepseek-chat';

function makeRequest(message, maxTokens = 2048) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify({
      model: MODEL,
      messages: [{ role: 'user', content: message }],
      max_tokens: maxTokens,
      temperature: parseFloat(process.env.DEEPSEEK_TEMPERATURE || '0.7')
    });

    const options = {
      hostname: API_URL,
      port: 443,
      path: '/v1/chat/completions',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${API_KEY}`,
        'Content-Length': data.length
      }
    };

    const req = https.request(options, (res) => {
      let body = '';
      res.on('data', (chunk) => body += chunk);
      res.on('end', () => {
        if (res.statusCode !== 200) {
          reject(new Error(`API returned ${res.statusCode}: ${body}`));
          return;
        }
        try {
          const response = JSON.parse(body);
          resolve(response);
        } catch (e) {
          reject(new Error(`Parse error: ${e.message}`));
        }
      });
    });

    req.on('error', reject);
    req.write(data);
    req.end();
  });
}

async function chat(prompt) {
  try {
    console.log('\n🤖 DeepSeek thinking...\n');
    const response = await makeRequest(prompt);

    console.log(response.choices[0].message.content);
    console.log(`\n📊 Tokens: ${response.usage.total_tokens} (prompt: ${response.usage.prompt_tokens}, completion: ${response.usage.completion_tokens})`);
    console.log(`⏱️  Model: ${response.model}\n`);
  } catch (error) {
    console.error('❌ Error:', error.message);
    process.exit(1);
  }
}

async function interactive() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    prompt: '💬 You: '
  });

  console.log('🧠 DeepSeek Interactive Chat');
  console.log('Type "exit" or "quit" to end session\n');

  rl.prompt();

  rl.on('line', async (line) => {
    const input = line.trim();

    if (input === 'exit' || input === 'quit') {
      console.log('👋 Goodbye!');
      rl.close();
      process.exit(0);
    }

    if (!input) {
      rl.prompt();
      return;
    }

    try {
      const response = await makeRequest(input);
      console.log(`\n🤖 DeepSeek: ${response.choices[0].message.content}`);
      console.log(`📊 ${response.usage.total_tokens} tokens\n`);
    } catch (error) {
      console.error('❌ Error:', error.message);
    }

    rl.prompt();
  });
}

// Main
const args = process.argv.slice(2);

if (args.length === 0) {
  console.log('Usage:');
  console.log('  deepseek-chat "your prompt here"');
  console.log('  deepseek-chat --interactive');
  console.log('  deepseek-chat --help');
  process.exit(0);
}

if (args[0] === '--help' || args[0] === '-h') {
  console.log('DeepSeek Chat CLI\n');
  console.log('Usage:');
  console.log('  deepseek-chat "prompt"         Send a single prompt');
  console.log('  deepseek-chat --interactive    Start interactive chat session');
  console.log('  deepseek-chat --help           Show this help\n');
  console.log('Environment variables:');
  console.log('  DEEPSEEK_API_KEY      API key (default: from config)');
  console.log('  DEEPSEEK_MODEL        Model name (default: deepseek-chat)');
  console.log('  DEEPSEEK_TEMPERATURE  Temperature 0.0-1.0 (default: 0.7)');
  process.exit(0);
}

if (args[0] === '--interactive' || args[0] === '-i') {
  interactive();
} else {
  const prompt = args.join(' ');
  chat(prompt);
}
