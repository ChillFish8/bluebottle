# Bluebottle UI components

Bluebottle's UI components and widgets for Iced.

## Core - Styling

> One of the most important parts of the UI is the colour theme and design language. This is generally inspired
> by Material UI, partially because I originally started building this UI in Flutter until I decided I hated writing Dart.

![style](/assets/design/style.png)

The UI itself tries to avoid using elevation for buttons and other inputs. Instead, we use elevation for the
sub-menues/modals.

Borders must always be fully rounded, making most things pilled shape while we add relatively minimal padding.