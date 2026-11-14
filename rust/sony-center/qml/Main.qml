import QtQuick
import QtQuick.Window
import SonyCenter

Window {
    visible: true
    width: 200; height: 100
    Text { anchors.centerIn: parent; text: "battery " + DeviceController.batteryLevel
        MouseArea { anchors.fill: parent; onClicked: DeviceController.bump() } }
    Component.onCompleted: { DeviceController.bump(); console.log("RESULT", DeviceController.batteryLevel); Qt.quit() }
}
