FROM ianpurton/rust-fullstack-devcontainer:latest AS development

COPY *.bash .

RUN cat *.bash >> ~/.bashrc && sudo rm *.bash
