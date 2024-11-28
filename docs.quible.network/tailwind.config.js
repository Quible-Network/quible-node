/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'selector',
  corePlugins: {
    preflight: false,
  },
  content: ["./src/**/*.{js,jsx}", "./docs/**/*.mdx"],
  theme: {
    extend: {
    colors: {
'quible-white': '#f7f3f7',
'quible-lightest': '#e1cce2',
'quible-lighter': '#d9c3de',
'quible-light': '#ccb4d5',
'quible-mildest': '#c6aad0',
'quible-milder': '#c2a3cc',
'quible-mild': '#c19dc9',
'quible-medium': '#bca0cb',
'quible-heavy': '#ba9eca',
'quible-heavier': '#b092c4',
'quible-heaviest': '#b08ec1',
'quible-deep': '#a98cc0',
'quible-deeper': '#a789bd',
'quible-deepest': '#9e81bb',
'quible-dark': '#9b7eb8',
'quible-darker': '#9073b3',
'quible-darkest': '#8b6eb0',
      'quible-black': '#6e4d99'
    },

    },
  },
  plugins: [],
};
