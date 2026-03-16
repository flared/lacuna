import pluginVue from "eslint-plugin-vue";
import tseslint from "typescript-eslint";

export default [
  ...pluginVue.configs["flat/base"],
  {
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
    rules: {
      "vue/no-restricted-static-attribute": [
        // We are aiming for a dead-simple app.
        // We want to stick to Vuetify components and props.
        // We will avoid customizing them with CSS classes or inline styles.
        "error",
        {
          key: "style",
          message: "Inline styles are not allowed. Prefer vanilla Vuetify or Tailwind classes.",
        },
      ],
    },
  },
];
