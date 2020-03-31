module.exports = {
    presets: [
        [
            '@babel/preset-env',
            {
                targets: {
                    node: '12',
                },
            },
        ],
    ],
    include: [
        "/target/pkg/(?!artemis-test).+\\.js$"
    ]
};