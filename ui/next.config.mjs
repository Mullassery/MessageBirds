/** @type {import('next').NextConfig} */
const nextConfig = {
  // @pycustomerhub/sdk is consumed as TypeScript source via the npm
  // workspace symlink (no separate build step) — Next needs to run it
  // through its own compiler like first-party code.
  transpilePackages: ["@pycustomerhub/sdk"],
};

export default nextConfig;
