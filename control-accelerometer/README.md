## Q&A

### Verify Multicast Handling on Devices
Multicast Group Membership: Ensure both devices are joining the multicast group correctly. On Linux-based systems (like Raspberry Pi), you can check this by looking at the multicast group memberships with the command:
bash

```
netstat -g
```

```
[Raspberry Pi]    SPI Interface    [ADXL345 #1 (SPI)]    [ADXL345 #2 (SPI)]    [ADXL345 #3 (SPI)]
+-------------+                    +----------------+    +----------------+    +----------------+
|             |                    |                |    |                |    |                |
|          GPIO 10 (MOSI)----------|SDA-------------|----|SDA-------------|----|SDA             |
|             |                    |                |    |                |    |                |
|          GPIO 9 (MISO)-----------|SDO-------------|----|SDO-------------|----|SDO             |
|             |                    |                |    |                |    |                |
|          GPIO 11 (SCLK)----------|SCL-------------|----|SCL-------------|----|SCL             |
|             |                    |                |    |                |    |                |
|          GPIO 17 (CS)------------|CS              |    |                |    |                |
|             |                    |                |    |                |    |                |
|          GPIO 22 (CS)----------------------------------|CS              |    |                |
|             |                    |                |    |                |    |                |
|          GPIO 6 (CS)--------------------------------------------------------|CS              |
|             |                    |                |    |                |    |                |
|          3.3V--------------------|VCC-------------|----|VCC-------------|----|VCC             |
|             |                    |                |    |                |    |                |
|          GND---------------------|GND-------------|----|GND-------------|----|GND             |
+-------------+                    +----------------+    +----------------+    +----------------+

```
