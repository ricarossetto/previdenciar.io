import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
export default defineConfig({plugins:[react()],test:{testTimeout:30000,hookTimeout:30000,fileParallelism:false,maxWorkers:1,environment:'jsdom',include:['tests/**/*.test.ts','tests/**/*.test.tsx'],setupFiles:['./tests/setup.ts']}});
