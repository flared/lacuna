import pluginVue from "eslint-plugin-vue";

export default [
  ...pluginVue.configs["flat/base"],
  {

    rules: {
      "vue/no-restricted-static-attribute": [
        // We are aiming for a dead-simple app.
        // We want to stick to Vuetify components and props.
        // We will avoid customizing them with CSS classes or inline styles.
        "error",
        {
          key: "class",
          message: "CSS classes are not allowed. Prefer vanilla Vuetify components.",
        },
        {
          key: "style",
          message: "Inline styles are not allowed. Prefer vanilla Vuetify components.",
        },
      ],
    },
  },
];
