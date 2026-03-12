import { defineConfig } from "vite-plus";

const ignorePatterns = ["files"];

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {
    ignorePatterns,
  },
  lint: { ignorePatterns, options: { typeAware: true, typeCheck: true } },
});
